// sequence and persist events for a single E3 request in the correct order
// TODO: spawn and store a ciphernode upon start and forward all events to it in order
// TODO: if the ciphernode fails restart the node by replaying all stored events back to it

use actix::prelude::*;

use crate::{Ciphernode, Data, EnclaveEvent, EventBus, Fhe};

pub struct CiphernodeSequencer {
    fhe: Addr<Fhe>,
    data: Addr<Data>,
    bus: Addr<EventBus>,
    ciphernode: Option<Addr<Ciphernode>>,
}

impl Actor for CiphernodeSequencer {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for CiphernodeSequencer {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let bus = self.bus.clone();
        let fhe = self.fhe.clone();
        let data = self.data.clone();
        let sink = self
            .ciphernode
            .get_or_insert_with(|| Ciphernode::new(bus, fhe, data).start());
        sink.do_send(msg);
    }
}
