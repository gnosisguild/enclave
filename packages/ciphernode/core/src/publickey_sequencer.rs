// sequence and persist events for a single E3 request in the correct order
// TODO: if the sequencer fails restart the node by replaying all stored events back to it

use actix::prelude::*;

use crate::{E3id, EnclaveEvent, EventBus, Fhe, PublicKeyAggregator, Sortition};

pub struct PublicKeySequencer {
    fhe: Addr<Fhe>,
    e3_id: E3id,
    bus: Addr<EventBus>,
    sortition:Addr<Sortition>,
    nodecount: usize,
    seed: u64,
    child: Option<Addr<PublicKeyAggregator>>,
}

impl PublicKeySequencer {
    pub fn new(fhe: Addr<Fhe>, e3_id: E3id, sortition:Addr<Sortition>,bus: Addr<EventBus>, nodecount: usize, seed:u64) -> Self {
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

impl Actor for PublicKeySequencer {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for PublicKeySequencer {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        let fhe = self.fhe.clone();
        let bus = self.bus.clone();
        let sortition = self.sortition.clone();
        let nodecount = self.nodecount;
        let e3_id = self.e3_id.clone();
        let seed = self.seed;
        let dest = self
            .child
            .get_or_insert_with(|| PublicKeyAggregator::new(fhe, bus, sortition, e3_id, nodecount, seed).start());
        dest.do_send(msg);
    }
}
