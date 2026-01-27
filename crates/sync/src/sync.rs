// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Addr, AsyncContext, Handler, Message};
use anyhow::Context;
use e3_events::{trap, BusHandle, EType, EventPublisher, EvmEventConfig, SyncEvmEvent, SyncStart};
struct Sync {
    bus: BusHandle,
    evm_config: Option<EvmEventConfig>,
    // net_config: NetEventConfig,
}

impl Sync {
    pub fn new(bus: &BusHandle, evm_config: EvmEventConfig) -> Self {
        Self {
            evm_config: Some(evm_config),
            bus: bus.clone(),
        }
    }

    pub fn setup(bus: &BusHandle, evm_config: EvmEventConfig) -> Addr<Self> {
        Self::new(bus, evm_config).start()
    }
}

impl Actor for Sync {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.notify(Bootstrap);
    }
}

impl Handler<SyncEvmEvent> for Sync {
    type Result = ();
    fn handle(&mut self, msg: SyncEvmEvent, ctx: &mut Self::Context) -> Self::Result {}
}

impl Handler<Bootstrap> for Sync {
    type Result = ();
    fn handle(&mut self, _: Bootstrap, ctx: &mut Self::Context) -> Self::Result {
        trap(EType::Sync, &self.bus.clone(), || {
            let evm_config = self.evm_config.take().context(
                "EvmEventConfig was not set likely Bootstrap was called more than once.",
            )?;

            // Fetch snapshot state
            self.bus.publish(SyncStart::new(ctx.address(), evm_config))
        })
    }
}

#[derive(Message)]
#[rtype("()")]
pub struct Bootstrap;
