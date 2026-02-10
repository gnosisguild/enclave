// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::SyncRepositoryFactory;
use actix::{Message, Recipient};
use anyhow::Result;
use e3_data::Repositories;
use e3_events::{
    AggregateConfig, AggregateId, BusHandle, CorrelationId, EffectsEnabled, EnclaveEvent,
    EventContextAccessors, EventPublisher, EventStoreQueryBy, EventStoreQueryResponse,
    EvmEventConfig, EvmEventConfigChain, HistoricalEvmEventsReceived, HistoricalEvmSyncStart,
    HistoricalNetEventsReceived, HistoricalNetSyncStart, SeqAgg, SyncEnded, Unsequenced,
};
use e3_utils::actix::channel as actix_toolbox;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    time::Duration,
};
use tokio::{sync::mpsc::Receiver, time::timeout};
use tracing::info;

pub async fn sync(
    bus: &BusHandle,
    default_config: &EvmEventConfig,
    repositories: &Repositories,
    aggregate_config: &AggregateConfig,
    eventstore: &Recipient<EventStoreQueryBy<SeqAgg>>,
) -> Result<()> {
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
    let net_config = snapshot.to_net_config();

    // 3. Load EventStore events since the sequence number found in the snapshot into memory.
    info!("Loading EventStore events...");
    let (tx, rx) = actix_toolbox::mpsc::<EventStoreQueryResponse>(256);
    eventstore.try_send(EventStoreQueryBy::<SeqAgg>::new(
        CorrelationId::new(),
        snapshot.to_sequence_map(),
        tx,
    ))?;
    let events =
        collect_eventstore_query_response(rx, snapshot.aggregates().len(), Duration::from_secs(5))
            .await;
    info!("{} EventStore events loaded.", events.len());

    info!("Replaying events to actors...");
    // 4. Replay the EventStore events to all listeners (except effects)
    for event in events {
        bus.event_bus().try_send(event)?;
    }
    info!("Events replayed.");

    // TODO: Detect open loops - incase we crashed in the middle of a request we need to play the
    // request event again once effects are on

    // 5. Load the historical evm events to memory from all chains
    info!("Loading historical blockchain events...");
    let (addr, rx) = actix_toolbox::mpsc::<HistoricalEvmEventsReceived>(256);
    bus.publish_without_context(HistoricalEvmSyncStart::new(addr, evm_config.clone()))?;
    let historical_evm_events =
        collect_historical_evm_events(rx, &evm_config, Duration::from_secs(30)).await;
    info!(
        "{} historical blockchain events loaded.",
        historical_evm_events.len()
    );

    // 6. Load the historical libp2p events to memory
    // info!("Loading historical libp2p events...");
    // let (addr, rx) = actix_toolbox::oneshot::<HistoricalNetEventsReceived>();
    // bus.publish_without_context(HistoricalNetSyncStart::new(addr, net_config.clone()))?;
    // let historical_net_events = rx.await?.events;
    // info!(
    //     "{} historical libp2p events loaded.",
    //     historical_net_events.len()
    // );

    // 7. Sort both the evm and libp2p events together by HLC timestamp
    let mut historical = historical_evm_events
        .into_iter()
        // .chain(historical_net_events)
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

    bus.publish_without_context(SyncEnded::new())?;
    info!("Sync finished.");
    // normal live operations

    Ok(())
}

pub async fn collect_historical_evm_events(
    mut receiver: Receiver<HistoricalEvmEventsReceived>,
    config: &EvmEventConfig,
    max_dur: Duration,
) -> Vec<EnclaveEvent<Unsequenced>> {
    // Get expected chain IDs from config
    let expected = config.chains();
    let mut received = HashSet::new();
    let mut results = Vec::new();

    let fut = async {
        while received.len() < expected.len() {
            if let Some(mut msg) = receiver.recv().await {
                // Only accept messages for expected chains we haven't received yet
                if expected.contains(&msg.chain_id) && !received.contains(&msg.chain_id) {
                    received.insert(msg.chain_id);
                    results.append(&mut msg.events);
                }
            } else {
                break;
            }
        }
    };

    if let Err(_) = timeout(max_dur, fut).await {
        for chain_id in expected.difference(&received) {
            eprintln!(
                "Error: Timeout waiting for historical events from chain {}",
                chain_id
            );
        }
    }

    results
}

pub async fn collect_eventstore_query_response(
    mut receiver: Receiver<EventStoreQueryResponse>,
    expected: usize,
    max_dur: Duration,
) -> Vec<EnclaveEvent> {
    let mut results = Vec::new();
    let mut received = 0;

    let collect = async {
        for _ in 0..expected {
            match receiver.recv().await {
                Some(msg) => {
                    results.extend(msg.into_events());
                    received += 1;
                }
                None => break,
            }
        }
    };

    if timeout(max_dur, collect).await.is_err() {
        eprintln!(
            "Error: Timeout waiting for historical events from {} aggregates",
            expected - received
        );
    }

    results
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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use e3_ciphernode_builder::EventSystem;
//     use e3_events::{EnclaveEvent, EventFactory};
//     use e3_events::{
//         EnclaveEventData, Event, EvmEventConfig, EvmEventConfigChain, GetEvents, TestEvent,
//     };
//     use std::collections::HashMap;
//     use std::time::Duration;
//     use tokio::time::sleep;
//
//     fn hlc_faucet(bus: &BusHandle, num: usize) -> Result<std::vec::IntoIter<u128>> {
//         let mut queue = Vec::new();
//         for _ in 0..num {
//             queue.push(bus.ts()?)
//         }
//
//         Ok(queue.into_iter())
//     }
//
//     async fn settle() {
//         sleep(Duration::from_millis(100)).await;
//     }
//
//     #[actix::test]
//     #[ignore]
//     async fn test_synchronizer_full_flow() -> Result<()> {
//         let _guard = e3_test_helpers::with_tracing("info");
//         // Setup event system and synchronizer
//         let system = EventSystem::new("test").with_fresh_bus();
//         let bus: BusHandle = system.handle()?;
//         let history_collector = bus.history();
//
//         // Configure test chains
//         let mut evm_config = EvmEventConfig::new();
//         evm_config.insert(1, EvmEventConfigChain::new(0));
//         evm_config.insert(2, EvmEventConfigChain::new(0));
//         let repositories = Repositories::in_mem();
//         let snapshot_buffer = system.buffer()?;
//         // Start synchronizer
//         let sync_addr = Synchronizer::setup(
//             &bus,
//             &evm_config,
//             &repositories,
//             &AggregateConfig::new(HashMap::new()),
//             &snapshot_buffer,
//         );
//         settle().await;
//
//         // Verify HistoricalEvmSyncStart was published
//         let history = history_collector
//             .send(GetEvents::<EnclaveEvent>::new())
//             .await?;
//         let sync_start_count = history
//             .into_iter()
//             .filter(|e| matches!(e.get_data(), EnclaveEventData::HistoricalEvmSyncStart(_)))
//             .count();
//         assert!(
//             sync_start_count > 0,
//             "HistoricalEvmSyncStart should be dispatched"
//         );
//
//         // Create test events with timestamps
//         let mut timelord = hlc_faucet(&bus, 100)?;
//
//         // Test events - timestamps generated in order
//         let h_2_1 = bus.event_from_remote_source(
//             EnclaveEventData::TestEvent(TestEvent::new("2-first", 1)),
//             None,
//             timelord.next().unwrap(),
//             Some(1),
//         )?;
//
//         let h_1_1 = bus.event_from_remote_source(
//             EnclaveEventData::TestEvent(TestEvent::new("1-first", 1)),
//             None,
//             timelord.next().unwrap(),
//             Some(1),
//         )?;
//
//         let h_1_2 = bus.event_from_remote_source(
//             EnclaveEventData::TestEvent(TestEvent::new("1-second", 2)),
//             None,
//             timelord.next().unwrap(),
//             Some(2),
//         )?;
//
//         let h_2_2 = bus.event_from_remote_source(
//             EnclaveEventData::TestEvent(TestEvent::new("2-second", 2)),
//             None,
//             timelord.next().unwrap(),
//             Some(2),
//         )?;
//
//         // Send events in mixed order to test sorting
//         sync_addr
//             .send(HistoricalEvmEventsReceived::new(vec![h_2_2, h_2_1], 2))
//             .await?;
//         sync_addr
//             .send(HistoricalEvmEventsReceived::new(vec![h_1_1, h_1_2], 1))
//             .await?;
//
//         settle().await;
//
//         // Get final event history and verify ordering
//         let full = history_collector
//             .send(GetEvents::<EnclaveEvent>::new())
//             .await?;
//         println!("full = {}", full.len());
//         let events: Vec<EnclaveEvent> = full
//             .into_iter()
//             .filter(|e| matches!(e.get_data(), EnclaveEventData::TestEvent(_)))
//             .collect();
//
//         let event_strings: Vec<String> = events
//             .into_iter()
//             .filter_map(|e| {
//                 if let EnclaveEventData::TestEvent(data) = e.into_data() {
//                     Some(data.msg)
//                 } else {
//                     None
//                 }
//             })
//             .collect();
//
//         // Events should be published in timestamp order
//         assert_eq!(
//             event_strings,
//             vec!["2-first", "1-first", "1-second", "2-second"]
//         );
//
//         Ok(())
//     }
// }
