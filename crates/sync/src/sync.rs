// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Addr, AsyncContext, Handler, Message};
use anyhow::Context;
use e3_events::{
    trap, BusHandle, EType, EventPublisher, EvmEvent, EvmEventConfig, SyncEnd, SyncEvmEvent,
    SyncStart,
};

// NOTE: This is a WIP. We need to synchronize events from EVM as well as libp2p

/// Manage the synchronization of events across.
pub struct Synchronizer {
    bus: BusHandle,
    evm_config: Option<EvmEventConfig>,
    evm_buffer: Vec<EvmEvent>,
    // net_config: NetEventConfig,
}

impl Synchronizer {
    pub fn new(bus: &BusHandle, evm_config: EvmEventConfig) -> Self {
        Self {
            evm_config: Some(evm_config),
            bus: bus.clone(),
            evm_buffer: Vec::new(),
        }
    }

    pub fn setup(bus: &BusHandle, evm_config: EvmEventConfig) -> Addr<Self> {
        Self::new(bus, evm_config).start()
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
                SyncEvmEvent::Event(event) => self.evm_buffer.push(event),
                // When we hear that sync is complete send all events on chain then publish SyncEnd
                SyncEvmEvent::HistoricalSyncComplete(_) => {
                    for evt in self.evm_buffer.drain(..) {
                        let (data, ts, _) = evt.split();
                        self.bus.publish_from_remote(data, ts)?;
                    }
                    self.bus.publish(SyncEnd::new())?;
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
