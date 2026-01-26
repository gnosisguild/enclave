// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::collections::HashMap;

use actix::{Actor, AsyncContext, Handler, Message};
use e3_events::{trap, BusHandle, EventPublisher, SyncEvmEvent, SyncStart};

struct Sync {
    bus: BusHandle,
}

impl Sync {}

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
    fn handle(&mut self, msg: Bootstrap, ctx: &mut Self::Context) -> Self::Result {
        trap(e3_events::EType::Sync, &self.bus.clone(), || {
            // Fetch snapshot state
            self.bus
                .publish(SyncStart::new(ctx.address(), HashMap::new()))
        })
    }
}

#[derive(Message)]
#[rtype("()")]
pub struct Bootstrap;
