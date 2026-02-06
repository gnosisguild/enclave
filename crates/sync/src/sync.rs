// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::collections::HashSet;

use actix::{Actor, Addr, AsyncContext, Handler, Message};
use anyhow::{Context, Result};
use e3_events::{
    trap, BusHandle, EType, EventPublisher, EvmEvent, EvmEventConfig, SyncEnd, SyncEvmEvent,
    SyncStart,
};
use tracing::info;

// NOTE: This is a WIP. We need to synchronize events from EVM as well as libp2p
type ChainId = u64;

/// Manage the synchronization of events across.
pub struct Synchronizer {
    bus: BusHandle,
    evm_config: Option<EvmEventConfig>,
    evm_buffer: Vec<EvmEvent>,
    evm_to_sync: HashSet<ChainId>,
    // net_config: NetEventConfig,
}

impl Synchronizer {
    pub fn new(bus: &BusHandle, evm_config: EvmEventConfig) -> Self {
        let evm_to_sync = evm_config.chains();
        Self {
            evm_config: Some(evm_config),
            bus: bus.clone(),
            evm_buffer: Vec::new(),
            evm_to_sync,
        }
    }

    pub fn setup(bus: &BusHandle, evm_config: EvmEventConfig) -> Addr<Self> {
        Self::new(bus, evm_config).start()
    }

    fn buffer_evm_event(&mut self, event: EvmEvent) {
        info!("buffer evm event({})", event.get_id());
        self.evm_buffer.push(event);
    }

    fn handle_sync_complete(&mut self, chain_id: u64) -> Result<()> {
        info!("handle sync complete for chain({})", chain_id);
        self.evm_to_sync.remove(&chain_id);
        info!("{} chains left to sync...", self.evm_to_sync.len());
        if self.evm_to_sync.is_empty() {
            self.handle_sync_end()?;
        }
        Ok(())
    }

    fn handle_sync_end(&mut self) -> Result<()> {
        info!("all chains synced draining to bus and running sync end");
        // Order all events (theoretically)
        self.evm_buffer.sort_by_key(|i| i.ts());

        // publish them in order
        for evt in self.evm_buffer.drain(..) {
            let (data, _, _) = evt.split();
            self.bus.publish(data)?; // Use publish here as historical events will be correctly
                                     // ordered as part of the preparatory process
        }
        self.bus.publish(SyncEnd::new())?;
        Ok(())
    }
}

impl Actor for Synchronizer {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.notify(Bootstrap);
    }
}

impl Handler<SyncEvmEvent> for Synchronizer {
    type Result = ();
    fn handle(&mut self, msg: SyncEvmEvent, _ctx: &mut Self::Context) -> Self::Result {
        trap(EType::Sync, &self.bus.clone(), || {
            match msg {
                // Buffer events as the sync actor receives them
                SyncEvmEvent::Event(event) => self.buffer_evm_event(event),
                // When we hear that sync is complete send all events on chain then publish SyncEnd
                SyncEvmEvent::HistoricalSyncComplete(chain_id) => {
                    self.handle_sync_complete(chain_id)?
                }
            };
            Ok(())
        })
    }
}

impl Handler<Bootstrap> for Synchronizer {
    type Result = ();
    fn handle(&mut self, _: Bootstrap, ctx: &mut Self::Context) -> Self::Result {
        trap(EType::Sync, &self.bus.clone(), || {
            let evm_config = self.evm_config.take().context(
                "EvmEventConfig was not set likely Bootstrap was called more than once.",
            )?;

            // TODO: Get information about what has and has not been synced then fire SyncStart
            self.bus.publish(SyncStart::new(ctx.address(), evm_config))
        })
    }
}

#[derive(Message)]
#[rtype("()")]
pub struct Bootstrap;

#[cfg(test)]
mod tests {
    use super::*;
    use e3_ciphernode_builder::EventSystem;
    use e3_events::EnclaveEvent;
    use e3_events::{
        CorrelationId, EnclaveEventData, Event, EvmEventConfig, EvmEventConfigChain, GetEvents,
        TestEvent,
    };
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
        // Setup event system and synchronizer
        let system = EventSystem::new("test").with_fresh_bus();
        let bus = system.handle()?;
        let history_collector = bus.history();

        // Configure test chains
        let mut evm_config = EvmEventConfig::new();
        evm_config.insert(1, EvmEventConfigChain::new(0));
        evm_config.insert(2, EvmEventConfigChain::new(0));

        // Start synchronizer
        let sync_addr = Synchronizer::setup(&bus, evm_config);
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
        let (chain_1, chain_2) = (1, 2);
        let (block_1, block_2) = (1, 2);

        // Test events - timestamps generated in order
        let h_2_1 = SyncEvmEvent::Event(EvmEvent::new(
            CorrelationId::new(),
            EnclaveEventData::TestEvent(TestEvent::new("2-first", 1)),
            block_1,
            timelord.next().unwrap(),
            chain_2,
        ));

        let h_1_1 = SyncEvmEvent::Event(EvmEvent::new(
            CorrelationId::new(),
            EnclaveEventData::TestEvent(TestEvent::new("1-first", 1)),
            block_1,
            timelord.next().unwrap(),
            chain_1,
        ));

        let h_1_2 = SyncEvmEvent::Event(EvmEvent::new(
            CorrelationId::new(),
            EnclaveEventData::TestEvent(TestEvent::new("1-second", 1)),
            block_2,
            timelord.next().unwrap(),
            chain_1,
        ));

        let h_2_2 = SyncEvmEvent::Event(EvmEvent::new(
            CorrelationId::new(),
            EnclaveEventData::TestEvent(TestEvent::new("2-second", 2)),
            block_2,
            timelord.next().unwrap(),
            chain_2,
        ));

        // Chain completion signals
        let hc_1 = SyncEvmEvent::HistoricalSyncComplete(chain_1);
        let hc_2 = SyncEvmEvent::HistoricalSyncComplete(chain_2);

        // Send events in mixed order to test sorting
        sync_addr.send(h_2_2).await?;
        sync_addr.send(h_2_1).await?;
        sync_addr.send(hc_2).await?;
        sync_addr.send(h_1_1).await?;
        sync_addr.send(h_1_2).await?;
        sync_addr.send(hc_1).await?;

        settle().await;

        // Get final event history and verify ordering
        let history = history_collector
            .send(GetEvents::<EnclaveEvent>::new())
            .await?;

        let events: Vec<EnclaveEvent> = history
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
