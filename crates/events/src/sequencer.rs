use actix::{Actor, Addr, Handler};

use crate::{EnclaveEvent, EventBus, Stored, Unstored};

pub struct Sequencer {
    bus: Addr<EventBus<EnclaveEvent<Stored>>>,
    seq: u64,
}

impl Sequencer {
    pub fn new(bus: &Addr<EventBus<EnclaveEvent<Stored>>>) -> Self {
        Self {
            bus: bus.clone(),
            seq: 0,
        }
    }
}

impl Actor for Sequencer {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent<Unstored>> for Sequencer {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent<Unstored>, _: &mut Self::Context) -> Self::Result {
        self.seq += 1;
        self.bus.do_send(msg.into_stored(self.seq))
    }
}
