// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::SyncRepositoryFactory;
use actix::{Message, Recipient};
use anyhow::{bail, Result};
use e3_data::Repositories;
use e3_events::{
    AggregateConfig, AggregateId, BusHandle, CorrelationId, E3id, EffectsEnabled, EnclaveEvent,
    EnclaveEventData, Event, EventContextAccessors, EventPublisher, EventStoreQueryBy,
    EventStoreQueryResponse, EventSubscriber, EventType, EvmEventConfig, EvmEventConfigChain,
    HistoricalEvmEventsReceived, HistoricalEvmSyncStart, HistoricalNetSyncStart, SeqAgg, SyncEnded,
    Unsequenced,
};
use e3_utils::actix::channel as actix_toolbox;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    time::Duration,
};
use tokio::sync::mpsc::Receiver;
use tracing::{info, warn};

fn is_infrastructure_event(event: &EnclaveEvent) -> bool {
    matches!(
        event.get_data(),
        EnclaveEventData::SyncEnded(_)
            | EnclaveEventData::EffectsEnabled(_)
            | EnclaveEventData::HistoricalEvmSyncStart(_)
    )
}

pub async fn sync(
    bus: &BusHandle,
    default_config: &EvmEventConfig,
    repositories: &Repositories,
    aggregate_config: &AggregateConfig,
    eventstore: &Recipient<EventStoreQueryBy<SeqAgg>>,
) -> Result<()> {
    // 0. start listening early for net ready
    let net_ready = bus.wait_for(EventType::NetReady);

    // 1. Load snapsshot metadata
    info!("Loading snapshot metadata...");
    let snapshot =
        SnapshotMeta::read_from_disk(aggregate_config.aggregates(), default_config, repositories)
            .await?;
    info!(
        "Snapshot metadata loaded for {} aggregates.",
        snapshot.aggregates().len()
    );

    // 2. Determine the evm blocks to read from based on the SnapshotMeta
    let evm_config = snapshot.to_evm_config();
    let _net_config = snapshot.to_net_config();

    // 3. Load EventStore events since the sequence number found in the snapshot into memory.
    info!("Loading EventStore events...");
    let (addr, rx) = actix_toolbox::oneshot::<EventStoreQueryResponse>();
    eventstore.try_send(EventStoreQueryBy::<SeqAgg>::new(
        CorrelationId::new(),
        snapshot.to_sequence_map(),
        addr,
    ))?;
    let events = rx.await?.into_events();
    info!("{} EventStore events loaded.", events.len());

    info!("Replaying events to actors...");
    // 4. Replay the EventStore events to all listeners (except effects).
    //    Skip infrastructure events (SyncEnded, EffectsEnabled, HistoricalEvmSyncStart) because
    //    they will be re-published by this sync process (steps 5, 8, 10). Replaying them here
    //    would poison the EventBus bloom-filter deduplication: the replayed event has the same
    //    EventId (payload hash) as the one we publish later, causing the later event to be
    //    silently dropped.  This is critical for SyncEnded, if the EvmChainGateway never
    //    receives it, the gateway stays in BufferUntilLive and all live EVM events are lost.
    for event in events {
        if is_infrastructure_event(&event) {
            continue;
        }
        bus.event_bus().try_send(event)?;
    }
    info!("Events replayed.");

    // TODO: Detect open loops - incase we crashed in the middle of a request we need to play the
    // request event again once effects are on

    // 5. Load the historical evm events to memory from all chains
    info!("Loading historical blockchain events...");
    let (addr, rx) = actix_toolbox::mpsc::<HistoricalEvmEventsReceived>(256);
    bus.publish_without_context(HistoricalEvmSyncStart::new(addr, evm_config.clone()))?;
    let historical_evm_events = collect_historical_evm_events(rx, &evm_config).await;
    info!(
        "{} historical blockchain events loaded.",
        historical_evm_events.len()
    );
    let net_config = find_net_hlc(&historical_evm_events);
    // 6. Load the historical libp2p events to memory
    info!("Waiting until NetReady...");
    net_ready.await?;
    info!("NetReady!");
    info!("Loading historical libp2p events...");
    // let (addr, rx) = actix_toolbox::oneshot::<HistoricalNetSyncEventsReceived>();
    let events_received = bus.wait_for(EventType::HistoricalNetSyncEventsReceived);
    bus.publish_without_context(HistoricalNetSyncStart::new(net_config.clone()))?;
    let EnclaveEventData::HistoricalNetSyncEventsReceived(event) =
        events_received.await?.into_data()
    else {
        bail!("failed to get HistoricalNetSyncEventsReceived");
    };
    let historical_net_events = event.events;
    info!(
        "{} historical libp2p events loaded.",
        historical_net_events.len()
    );

    // 7. Sort both the evm and libp2p events together by HLC timestamp
    let mut historical = historical_evm_events
        .into_iter()
        .chain(historical_net_events)
        .collect::<Vec<_>>();

    historical.sort_by_key(|event| event.ts());
    info!("Historical events sorted.");

    // 8. Enable effects
    bus.publish_without_context(EffectsEnabled::new())?;
    info!("Effects enabled");

    // 9. Publish the new sorted events to the eventstore
    info!("Publishing historical events to actors...");
    for event in historical {
        bus.naked_dispatch(event);
    }
    info!("Historical events published.");

    // 10. Publish the SyncEnded event
    info!("Publishing SyncEnded event...");
    bus.publish_without_context(SyncEnded::new())?;
    info!("Sync finished.");
    // normal live operations

    Ok(())
}

pub async fn collect_historical_evm_events(
    mut receiver: Receiver<HistoricalEvmEventsReceived>,
    config: &EvmEventConfig,
) -> Vec<EnclaveEvent<Unsequenced>> {
    // Get expected chain IDs from config
    let expected = config.chains();
    let mut received = HashSet::new();
    let mut results = Vec::new();
    let progress_interval = Duration::from_secs(30);

    while received.len() < expected.len() {
        match tokio::time::timeout(progress_interval, receiver.recv()).await {
            Ok(Some(mut msg)) => {
                if expected.contains(&msg.chain_id) && !received.contains(&msg.chain_id) {
                    info!(
                        chain_id = msg.chain_id,
                        events = msg.events.len(),
                        chains_received = received.len() + 1,
                        chains_expected = expected.len(),
                        "Received historical events from chain"
                    );
                    received.insert(msg.chain_id);
                    results.append(&mut msg.events);
                }
            }
            Ok(None) => {
                // Channel closed — sender dropped
                warn!("Historical events channel closed before all chains reported");
                break;
            }
            Err(_) => {
                // Not a failure — just a progress heartbeat
                let remaining: Vec<_> = expected.difference(&received).collect();
                info!(
                    ?remaining,
                    "Still waiting for historical events from chains"
                );
                continue;
            }
        }
    }

    results
}

fn find_net_hlc(events: &[EnclaveEvent<Unsequenced>]) -> BTreeMap<AggregateId, u128> {
    // find all E3s that are closed
    let e3s: Vec<E3id> = events
        .iter()
        .filter_map(|e| match e.get_data() {
            EnclaveEventData::E3Failed(d) => Some(d.e3_id.clone()),
            EnclaveEventData::E3RequestComplete(d) => Some(d.e3_id.clone()),
            _ => None,
        })
        .collect();
    events
        .to_vec()
        .into_iter()
        .filter(|e| e.get_e3_id().map_or(true, |id| !e3s.contains(&id)))
        .fold(BTreeMap::new(), |mut acc, e| {
            acc.entry(e.aggregate_id())
                .and_modify(|ts| *ts = (*ts).max(e.ts()))
                .or_insert(e.ts());
            acc
        })
}

/// Latest event information in store
#[derive(Clone)]
pub struct AggregateState {
    ts: u128,
    aggregate_id: AggregateId,
    seq: u64,
    block: u64,
}

#[derive(Clone)]
pub struct SnapshotMeta {
    aggregate_state: Vec<AggregateState>,
}

impl SnapshotMeta {
    /// Load the SnapshotMeta from the Snapshot on disk
    pub async fn read_from_disk(
        ids: Vec<AggregateId>,
        initial_evm_config: &EvmEventConfig,
        repositories: &Repositories,
    ) -> Result<Self> {
        let mut aggregate_state = Vec::new();
        for aggregate_id in ids {
            let deploy_block = aggregate_id
                .to_chain_id()
                .and_then(|chain_id| initial_evm_config.deploy_block(chain_id))
                .unwrap_or(0);
            let seq_repo = repositories.aggregate_seq(aggregate_id);
            let block_repo = repositories.aggregate_block(aggregate_id);
            let ts_repo = repositories.aggregate_ts(aggregate_id);
            let seq = seq_repo.read().await?.unwrap_or(0);
            let block = block_repo.read().await?.unwrap_or(deploy_block);
            let ts = ts_repo.read().await?.unwrap_or(0);
            let agg_state = AggregateState {
                aggregate_id,
                seq,
                block,
                ts,
            };
            aggregate_state.push(agg_state);
        }

        Ok(Self { aggregate_state })
    }

    /// Return an EvmEventConfig based on the SnapshotMeta
    pub fn to_evm_config(&self) -> EvmEventConfig {
        let map: BTreeMap<u64, EvmEventConfigChain> = self
            .aggregate_state
            .iter()
            .map(|s| (s.aggregate_id.to_chain_id(), s.block))
            .filter_map(|s| {
                if let Some(chain) = s.0 {
                    Some((chain, EvmEventConfigChain::new(s.1)))
                } else {
                    None
                }
            })
            .collect();
        EvmEventConfig::from_config(map)
    }

    pub fn to_net_config(&self) -> BTreeMap<AggregateId, u128> {
        self.aggregate_state
            .iter()
            .map(|s| (s.aggregate_id, s.ts))
            .collect()
    }

    /// Return a map between AggregateIds and Sequence
    pub fn to_sequence_map(&self) -> HashMap<AggregateId, u64> {
        self.aggregate_state
            .iter()
            .fold(HashMap::new(), |mut acc, item| {
                acc.insert(item.aggregate_id, item.seq);
                acc
            })
    }

    pub fn aggregates(&self) -> Vec<AggregateId> {
        self.aggregate_state
            .iter()
            .map(|s| s.aggregate_id)
            .collect()
    }
}

#[derive(Message)]
#[rtype("()")]
pub struct Bootstrap;

#[derive(Message)]
#[rtype("()")]
pub struct SnapshotLoaded {
    pub snapshot: SnapshotMeta,
}
impl SnapshotLoaded {
    pub fn new(snapshot: SnapshotMeta) -> Self {
        Self { snapshot }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use e3_ciphernode_builder::EventSystem;
    use e3_events::{
        E3Failed, E3RequestComplete, E3Stage, E3id, EffectsEnabled, EnclaveEvent, EnclaveEventData,
        Event, EvmEventConfig, FailureReason, HistoricalEvmSyncStart, SyncEnded, TakeEvents,
        Unsequenced,
    };

    fn make_historical_evm_sync_start() -> HistoricalEvmSyncStart {
        HistoricalEvmSyncStart {
            evm_config: EvmEventConfig::new(),
            sender: None,
        }
    }

    #[test]
    fn infrastructure_events_are_detected() {
        let sync_ended = EnclaveEvent::<Unsequenced>::test_event("sync")
            .data(SyncEnded::new())
            .seq(1)
            .build();
        let effects_enabled = EnclaveEvent::<Unsequenced>::test_event("fx")
            .data(EffectsEnabled::new())
            .seq(2)
            .build();
        let evm_sync_start = EnclaveEvent::<Unsequenced>::test_event("evm")
            .data(make_historical_evm_sync_start())
            .seq(3)
            .build();
        let test_event = EnclaveEvent::<Unsequenced>::test_event("hello")
            .id(42)
            .seq(4)
            .build();

        assert!(is_infrastructure_event(&sync_ended));
        assert!(is_infrastructure_event(&effects_enabled));
        assert!(is_infrastructure_event(&evm_sync_start));
        assert!(!is_infrastructure_event(&test_event));
    }

    #[actix::test]
    async fn infrastructure_events_are_filtered_during_replay() -> anyhow::Result<()> {
        let system = EventSystem::new().with_fresh_bus();
        let bus = system.handle()?.enable("test-sync-replay");
        let history = bus.history();

        let events: Vec<EnclaveEvent> = vec![
            EnclaveEvent::<Unsequenced>::test_event("before")
                .id(1)
                .seq(1)
                .build(),
            EnclaveEvent::<Unsequenced>::test_event("sync")
                .data(SyncEnded::new())
                .seq(2)
                .build(),
            EnclaveEvent::<Unsequenced>::test_event("fx")
                .data(EffectsEnabled::new())
                .seq(3)
                .build(),
            EnclaveEvent::<Unsequenced>::test_event("evm")
                .data(make_historical_evm_sync_start())
                .seq(4)
                .build(),
            EnclaveEvent::<Unsequenced>::test_event("after")
                .id(2)
                .seq(5)
                .build(),
        ];

        for event in events {
            if is_infrastructure_event(&event) {
                continue;
            }
            bus.event_bus().try_send(event)?;
        }

        let received = history.send(TakeEvents::new(2)).await?;

        let event_types: Vec<&'static str> = received
            .iter()
            .map(|e| match e.get_data() {
                EnclaveEventData::TestEvent(_) => "TestEvent",
                EnclaveEventData::SyncEnded(_) => "SyncEnded",
                EnclaveEventData::EffectsEnabled(_) => "EffectsEnabled",
                EnclaveEventData::HistoricalEvmSyncStart(_) => "HistoricalEvmSyncStart",
                _ => "other",
            })
            .collect();

        assert_eq!(event_types, vec!["TestEvent", "TestEvent"]);

        let msgs: Vec<String> = received
            .iter()
            .filter_map(|e| {
                if let EnclaveEventData::TestEvent(t) = e.get_data() {
                    Some(t.msg.clone())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(msgs, vec!["before", "after"]);
        Ok(())
    }

    #[test]
    fn test_find_net_hlc() {
        let closed_1 = E3id::new("1", 1);
        let closed_2 = E3id::new("2", 2);
        let open_1 = E3id::new("3", 3);
        let open_2 = E3id::new("4", 4);

        let events = vec![
            // closed e3s -> should be filtered out
            EnclaveEvent::<Unsequenced>::test_event("a")
                .e3_id(closed_1.clone())
                .ts(1000)
                .build(),
            EnclaveEvent::<Unsequenced>::test_event("a")
                .e3_id(closed_1.clone())
                .ts(2000)
                .build(),
            EnclaveEvent::<Unsequenced>::test_event("complete")
                .data(E3RequestComplete {
                    e3_id: closed_1.clone(),
                })
                .ts(3000)
                .build(),
            EnclaveEvent::<Unsequenced>::test_event("b")
                .e3_id(closed_2.clone())
                .ts(1500)
                .build(),
            EnclaveEvent::<Unsequenced>::test_event("failed")
                .data(E3Failed {
                    e3_id: closed_2.clone(),
                    failed_at_stage: E3Stage::CommitteeFinalized,
                    reason: FailureReason::InsufficientCommitteeMembers,
                })
                .ts(2500)
                .build(),
            // open e3s -> should be kept
            EnclaveEvent::<Unsequenced>::test_event("c")
                .e3_id(open_1.clone())
                .ts(4000)
                .build(),
            EnclaveEvent::<Unsequenced>::test_event("c")
                .e3_id(open_1.clone())
                .ts(5000)
                .build(),
            EnclaveEvent::<Unsequenced>::test_event("d")
                .e3_id(open_2.clone())
                .ts(6000)
                .build(),
            // no e3_id -> aggregate 0, always kept
            EnclaveEvent::<Unsequenced>::test_event("e")
                .ts(7000)
                .build(),
            EnclaveEvent::<Unsequenced>::test_event("e")
                .ts(8000)
                .build(),
        ];

        let result = find_net_hlc(&events);

        // closed e3s excluded
        assert!(!result.contains_key(&AggregateId::new(1)));
        assert!(!result.contains_key(&AggregateId::new(2)));

        // open e3s kept with max ts
        assert_eq!(result[&AggregateId::new(3)], 5000);
        assert_eq!(result[&AggregateId::new(4)], 6000);

        // no-e3 events kept with max ts
        assert_eq!(result[&AggregateId::new(0)], 8000);

        assert_eq!(result.len(), 3);
    }
}
