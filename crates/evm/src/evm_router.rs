use crate::events::{EnclaveEvmEvent, EvmEventProcessor, EvmLog};
use actix::{Actor, Addr, Handler};
use alloy_primitives::Address;
use std::collections::HashMap;

/// Directs EnclaveEvmEvent::Log events to the correct upstream processors. Drops all other event
/// types
pub struct EvmRouter {
    routing_table: HashMap<Address, EvmEventProcessor>,
}

impl EvmRouter {
    pub fn new(routing_table: HashMap<Address, EvmEventProcessor>) -> Self {
        Self { routing_table }
    }

    pub fn setup(routing_table: HashMap<Address, EvmEventProcessor>) -> Addr<Self> {
        let addr = Self::new(routing_table).start();
        addr
    }
}

impl Actor for EvmRouter {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvmEvent> for EvmRouter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvmEvent, ctx: &mut Self::Context) -> Self::Result {
        let EnclaveEvmEvent::Log(EvmLog { log, .. }) = msg.clone() else {
            return;
        };
        let Some(dest) = self.routing_table.get(&log.address()) else {
            return;
        };

        dest.do_send(msg);
    }
}

pub struct EvmHub {
    nexts: Vec<EvmEventProcessor>,
}

impl EvmHub {
    pub fn new(nexts: Vec<EvmEventProcessor>) -> Self {
        Self { nexts }
    }

    pub fn setup(nexts: Vec<EvmEventProcessor>) -> Addr<Self> {
        let addr = Self::new(nexts).start();
        addr
    }
}

impl Actor for EvmHub {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvmEvent> for EvmHub {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvmEvent, ctx: &mut Self::Context) -> Self::Result {
        let EnclaveEvmEvent::Log { .. } = msg.clone() else {
            return;
        };

        for next in self.nexts.clone() {
            next.do_send(msg.clone());
        }
    }
}
