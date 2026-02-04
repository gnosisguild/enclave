// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::SyncRepositoryFactory;
use actix::{Actor, Addr, AsyncContext, Handler, Message};
use anyhow::{Context, Result};
use e3_data::Repositories;
use e3_events::{
    trap, trap_fut, AggregateConfig, AggregateId, BusHandle, EType, EnclaveEvent,
    EventContextAccessors, EventPublisher, EvmEventConfig, EvmEventConfigChain,
    EvmSyncEventsReceived, SnapshotBuffer, SyncEnd, Unsequenced,
};
use std::collections::{BTreeMap, HashSet};
use tracing::info;

// NOTE: This is a WIP. We need to synchronize events from EVM as well as libp2p
type ChainId = u64;

/// Manage the synchronization of events across.
pub struct Synchronizer {
    bus: BusHandle,
    evm_config: Option<EvmEventConfig>,
    evm_events: Vec<EnclaveEvent<Unsequenced>>,
    evm_to_sync: HashSet<ChainId>,
    repositories: Repositories,
    snapshot_buffer: Addr<SnapshotBuffer>,
    // net_config: NetEventConfig,
    aggregate_config: AggregateConfig,
}

impl Synchronizer {
    pub fn new(
        bus: BusHandle,
        evm_config: EvmEventConfig,
        repositories: Repositories,
        aggregate_config: AggregateConfig,
        snapshot_buffer: Addr<SnapshotBuffer>,
    ) -> Self {
        let evm_to_sync = evm_config.chains();
        Self {
            evm_config: Some(evm_config),
            bus,
            evm_to_sync,
            evm_events: Vec::new(),
            repositories,
            aggregate_config,
            snapshot_buffer,
        }
    }

    pub fn setup(
        bus: &BusHandle,
        evm_config: &EvmEventConfig,
        repositories: &Repositories,
        aggregate_config: &AggregateConfig,
        snapshot_buffer: &Addr<SnapshotBuffer>,
    ) -> Addr<Self> {
        Self::new(
            bus.clone(),
            evm_config.clone(),
            repositories.clone(),
            aggregate_config.clone(),
            snapshot_buffer.clone(),
        )
        .start()
    }

    fn handle_evm_sync_events_received(&mut self, mut msg: EvmSyncEventsReceived) -> Result<()> {
        let chain_id = msg.chain_id;
        info!("handle sync complete for chain({})", chain_id);
        self.evm_to_sync.remove(&chain_id);
        self.evm_events.append(&mut msg.events);
        info!("{} chains left to sync...", self.evm_to_sync.len());
        if self.evm_to_sync.is_empty() {
            self.sort_and_finalize()?;
        }
        Ok(())
    }

    fn sort_and_finalize(&mut self) -> Result<()> {
        info!("all chains synced draining to bus and running sync end");
        // Order all events (theoretically)
        self.evm_events.sort_by_key(|i| i.ts());

        // publish them in order
        for evt in self.evm_events.drain(..) {
            self.bus.naked_dispatch(evt);
        }
        self.bus.publish_without_context(SyncEnd::new())?;
        Ok(())
    }
}

impl Actor for Synchronizer {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.notify(Bootstrap);
    }
}

impl Handler<EvmSyncEventsReceived> for Synchronizer {
    type Result = ();
    fn handle(&mut self, msg: EvmSyncEventsReceived, _: &mut Self::Context) -> Self::Result {
        trap(EType::Sync, &self.bus.clone(), || {
            self.handle_evm_sync_events_received(msg)?;
            Ok(())
        })
    }
}

impl Handler<Bootstrap> for Synchronizer {
    type Result = actix::ResponseFuture<()>;
    fn handle(&mut self, _: Bootstrap, ctx: &mut Self::Context) -> Self::Result {
        let address = ctx.address();
        let repositories = self.repositories.clone();
        let evm_config = self.evm_config.take();
        let aggregates = self.aggregate_config.aggregates();
        let bus = self.bus.clone();
        trap_fut(
            EType::Sync,
            &self.bus.clone(),
            handle_bootstrap(bus, address, repositories, evm_config, aggregates),
        )
    }
}

async fn handle_bootstrap(
    bus: BusHandle,
    address: Addr<Synchronizer>,
    repositories: Repositories,
    evm_config: Option<EvmEventConfig>,
    aggregates: Vec<AggregateId>,
) -> Result<()> {
    let evm_config = evm_config
        .context("EvmEventConfig was not set likely Bootstrap was called more than once.")?;
    // ============================================================
    // Phase 1: Load Snapshot
    // ============================================================

    // 1.1 Read snapshot from disk (may not exist on first boot)
    let snapshot = SnapshotMeta::read_from_disk(aggregates, evm_config, repositories).await?;

    // 1.2 Extract state, last_applied_hlc, last_block_number (use defaults if no snapshot)

    // 1.4 Pause WriteBuffer streaming (don't write replayed mutations to disk)

    // ============================================================
    // Phase 2: Replay Missed Events
    // ============================================================

    // 2.1 Query EventStore for all events WHERE hlc > last_applied_hlc ORDER BY hlc

    // 2.2 For each event:
    //     - Route to appropriate actor by aggregate_id
    //     - Apply event mutation (in-memory only)
    //     - Track highest block_number seen (if event has one)

    // ============================================================
    // Phase 3: Determine Blockchain Resume Point
    // ============================================================

    // 3.1 Get highest block_number from replayed events (if any had block numbers)

    // 3.2 Fall back to snapshot.last_block_number if no blockchain events replayed

    // 3.3 Calculate resume_block = max(config.deploy_block, highest_block + 1)

    // ============================================================
    // Phase 4: Resume Normal Operation
    // ============================================================

    // 4.1 Update WriteBuffer watermark to current HLC

    // 4.2 Resume WriteBuffer streaming (mutations now flow to disk)

    // 4.3 Subscribe to blockchain from resume_block

    // 4.4 Return Ok / start event loop
    // Get the sequences for each aggregate
    // bus.publish_without_context(SyncStart::new(address, aggregate_states.as_evm_config()))?;
    Ok(())
}

/// Latest event information in store
pub struct AggregateState {
    ts: u128,
    aggregate_id: AggregateId,
    seq: u64,
    block: u64,
}

struct SnapshotMeta {
    aggregate_state: Vec<AggregateState>,
}

impl SnapshotMeta {
    pub async fn read_from_disk(
        ids: Vec<AggregateId>,
        initial_evm_config: EvmEventConfig,
        repositories: Repositories,
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

    pub fn as_evm_config(&self) -> EvmEventConfig {
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
}

#[derive(Message)]
#[rtype("()")]
pub struct Bootstrap;

#[cfg(test)]
mod tests {
    use super::*;
    use actix::io::WriteHandler;
    use e3_ciphernode_builder::EventSystem;
    use e3_events::{EnclaveEvent, EventFactory};
    use e3_events::{
        EnclaveEventData, Event, EvmEventConfig, EvmEventConfigChain, GetEvents, TestEvent,
    };
    use std::collections::HashMap;
    use std::time::Duration;
    use tokio::time::sleep;

    fn hlc_faucet(bus: &BusHandle, num: usize) -> Result<std::vec::IntoIter<u128>> {
        let mut queue = Vec::new();
        for _ in 0..num {
            queue.push(bus.ts()?)
        }

        Ok(queue.into_iter())
    }

    async fn settle() {
        sleep(Duration::from_millis(100)).await;
    }

    #[actix::test]
    async fn test_synchronizer_full_flow() -> Result<()> {
        let _guard = e3_test_helpers::with_tracing("info");
        // Setup event system and synchronizer
        let system = EventSystem::new("test").with_fresh_bus();
        let bus: BusHandle = system.handle()?;
        let history_collector = bus.history();

        // Configure test chains
        let mut evm_config = EvmEventConfig::new();
        evm_config.insert(1, EvmEventConfigChain::new(0));
        evm_config.insert(2, EvmEventConfigChain::new(0));
        let repositories = Repositories::in_mem();
        let snapshot_buffer = WriteBuffer::new().start();
        // Start synchronizer
        let sync_addr = Synchronizer::setup(
            &bus,
            &evm_config,
            &repositories,
            &AggregateConfig::new(HashMap::new()),
            &snapshot_buffer,
        );
        settle().await;

        // Verify SyncStart was published
        let history = history_collector
            .send(GetEvents::<EnclaveEvent>::new())
            .await?;
        let sync_start_count = history
            .into_iter()
            .filter(|e| matches!(e.get_data(), EnclaveEventData::SyncStart(_)))
            .count();
        assert!(sync_start_count > 0, "SyncStart should be dispatched");

        // Create test events with timestamps
        let mut timelord = hlc_faucet(&bus, 100)?;

        // Test events - timestamps generated in order
        let h_2_1 = bus.event_from_remote_source(
            EnclaveEventData::TestEvent(TestEvent::new("2-first", 1)),
            None,
            timelord.next().unwrap(),
            Some(1),
        )?;

        let h_1_1 = bus.event_from_remote_source(
            EnclaveEventData::TestEvent(TestEvent::new("1-first", 1)),
            None,
            timelord.next().unwrap(),
            Some(1),
        )?;

        let h_1_2 = bus.event_from_remote_source(
            EnclaveEventData::TestEvent(TestEvent::new("1-second", 2)),
            None,
            timelord.next().unwrap(),
            Some(2),
        )?;

        let h_2_2 = bus.event_from_remote_source(
            EnclaveEventData::TestEvent(TestEvent::new("2-second", 2)),
            None,
            timelord.next().unwrap(),
            Some(2),
        )?;

        // Send events in mixed order to test sorting
        sync_addr
            .send(EvmSyncEventsReceived::new(vec![h_2_2, h_2_1], 2))
            .await?;
        sync_addr
            .send(EvmSyncEventsReceived::new(vec![h_1_1, h_1_2], 1))
            .await?;

        settle().await;

        // Get final event history and verify ordering
        let full = history_collector
            .send(GetEvents::<EnclaveEvent>::new())
            .await?;
        println!("full = {}", full.len());
        let events: Vec<EnclaveEvent> = full
            .into_iter()
            .filter(|e| matches!(e.get_data(), EnclaveEventData::TestEvent(_)))
            .collect();

        let event_strings: Vec<String> = events
            .into_iter()
            .filter_map(|e| {
                if let EnclaveEventData::TestEvent(data) = e.into_data() {
                    Some(data.msg)
                } else {
                    None
                }
            })
            .collect();

        // Events should be published in timestamp order
        assert_eq!(
            event_strings,
            vec!["2-first", "1-first", "1-second", "2-second"]
        );

        Ok(())
    }
}
