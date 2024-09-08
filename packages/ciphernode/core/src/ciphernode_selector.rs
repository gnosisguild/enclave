use actix::prelude::*;

use crate::{CiphernodeSelected, CommitteeRequested, EnclaveEvent, EventBus, Subscribe};

pub struct CiphernodeSelector {
    bus: Addr<EventBus>,
}

impl Actor for CiphernodeSelector {
    type Context = Context<Self>;
}

impl CiphernodeSelector {
    pub fn new(bus: Addr<EventBus>) -> Self {
        Self { bus }
    }

    pub fn attach(bus: Addr<EventBus>) -> Addr<Self> {
        let addr = CiphernodeSelector::new(bus.clone()).start();

        bus.do_send(Subscribe::new(
            "CommitteeRequested",
            addr.clone().recipient(),
        ));

        addr
    }
}

impl Handler<EnclaveEvent> for CiphernodeSelector {
    type Result = ();

    fn handle(&mut self, event: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match event {
            EnclaveEvent::CommitteeRequested { data, .. } => {
                // TODO: ask Sortition module whether registered node has been selected
                self.bus.do_send(EnclaveEvent::from(CiphernodeSelected {
                    e3_id: data.e3_id,
                    nodecount: data.nodecount,
                    threshold: data.threshold,
                }))
            }
            _ => (),
        }
    }
}
