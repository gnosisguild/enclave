// sequence and persist events for a single E3 request in the correct order
// TODO: spawn and store a ciphernode upon start and forward all events to it in order
// TODO: if the ciphernode fails restart the node by replaying all stored events back to it

use actix::prelude::*;

use crate::{Data, E3id, EnclaveEvent, EventBus, Fhe, PublicKeyAggregator};

pub struct PublicKeySequencer {
    fhe: Addr<Fhe>,
    e3_id: E3id,
    bus: Addr<EventBus>,
    nodecount: usize,
    child: Option<Addr<PublicKeyAggregator>>,
}

impl PublicKeySequencer {
    fn new(fhe: Addr<Fhe>, e3_id: E3id, bus: Addr<EventBus>, nodecount: usize) -> Self {
        Self {
            fhe,
            e3_id,
            bus,
            nodecount,
            child: None,
        }
    }
}

impl Actor for PublicKeySequencer {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for PublicKeySequencer {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let fhe = self.fhe.clone();
        let bus = self.bus.clone();
        let nodecount = self.nodecount;
        let e3_id = self.e3_id.clone();
        let sink = self
            .child
            .get_or_insert_with(|| PublicKeyAggregator::new(fhe, bus, e3_id, nodecount).start());
        sink.do_send(msg);
    }
}
