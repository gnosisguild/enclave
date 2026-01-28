// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::collections::{HashMap, HashSet};

use actix::{Actor, Addr, AsyncContext, Handler, Message};
use anyhow::{Context, Result};
use e3_events::{
    trap, BusHandle, EType, EnclaveEvent, EventPublisher, EvmEvent, EvmEventConfig, SyncEnd,
    SyncEvmEvent, SyncStart,
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
    fn handle(&mut self, msg: SyncEvmEvent, ctx: &mut Self::Context) -> Self::Result {
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
