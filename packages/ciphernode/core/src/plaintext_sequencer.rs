// sequence and persist events for a single E3 request in the correct order
// TODO: spawn and store a ciphernode upon start and forward all events to it in order
// TODO: if the ciphernode fails restart the node by replaying all stored events back to it

use actix::prelude::*;

use crate::{E3id, EnclaveEvent, EventBus, Fhe, PlaintextAggregator, Sortition};

pub struct PlaintextSequencer {
    fhe: Addr<Fhe>,
    e3_id: E3id,
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
    nodecount: usize,
    seed: u64,
    child: Option<Addr<PlaintextAggregator>>,
}
impl PlaintextSequencer {
    pub fn new(
        fhe: Addr<Fhe>,
        e3_id: E3id,
        bus: Addr<EventBus>,
        sortition: Addr<Sortition>,
        nodecount: usize,
        seed: u64,
    ) -> Self {
        Self {
            fhe,
            e3_id,
            bus,
            sortition,
            seed,
            nodecount,
            child: None,
        }
    }
}
impl Actor for PlaintextSequencer {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for PlaintextSequencer {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        let fhe = self.fhe.clone();
        let bus = self.bus.clone();
        let sortition = self.sortition.clone();
        let nodecount = self.nodecount;
        let e3_id = self.e3_id.clone();
        let seed = self.seed;
        let sink = self.child.get_or_insert_with(|| {
            PlaintextAggregator::new(fhe, bus, sortition, e3_id, nodecount, seed).start()
        });
        sink.do_send(msg);
    }
}
