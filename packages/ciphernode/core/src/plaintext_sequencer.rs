// sequence and persist events for a single E3 request in the correct order
// TODO: spawn and store a ciphernode upon start and forward all events to it in order
// TODO: if the ciphernode fails restart the node by replaying all stored events back to it

use actix::prelude::*;

use crate::{E3id, EnclaveEvent, EventBus, Fhe, PlaintextAggregator};

pub struct PlaintextSequencer {
    fhe: Addr<Fhe>,
    e3_id: E3id,
    bus: Addr<EventBus>,
    nodecount: usize,
    child: Option<Addr<PlaintextAggregator>>,
}

impl Actor for PlaintextSequencer {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for PlaintextSequencer {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let fhe = self.fhe.clone();
        let bus = self.bus.clone();
        let nodecount = self.nodecount;
        let e3_id = self.e3_id.clone();
        let sink = self
            .child
            .get_or_insert_with(|| PlaintextAggregator::new(fhe, bus, e3_id, nodecount).start());
        sink.do_send(msg);
    }
}
