use actix::prelude::*;
use alloy_primitives::Address;

use crate::{
    CiphernodeSelected, EnclaveEvent, EventBus, GetHasNode, Sortition, Subscribe,
};

pub struct CiphernodeSelector {
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
    address: Address, 
}

impl Actor for CiphernodeSelector {
    type Context = Context<Self>;
}

impl CiphernodeSelector {
    pub fn new(bus: Addr<EventBus>, sortition: Addr<Sortition>, address: Address) -> Self {
        Self {
            bus,
            sortition,
            address,
        }
    }

    pub fn attach(bus: Addr<EventBus>, sortition: Addr<Sortition>, address: Address) -> Addr<Self> {
        let addr = CiphernodeSelector::new(bus.clone(), sortition, address).start();

        bus.do_send(Subscribe::new(
            "CommitteeRequested",
            addr.clone().recipient(),
        ));

        addr
    }
}

impl Handler<EnclaveEvent> for CiphernodeSelector {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, event: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        let address = self.address;
        let sortition = self.sortition.clone();
        let bus = self.bus.clone();

        Box::pin(async move {
            let EnclaveEvent::CommitteeRequested { data, .. } = event else {
                return;
            };

            let seed = data.sortition_seed;
            let size = data.nodecount;

            if let Ok(is_selected) = sortition
                .send(GetHasNode {
                    seed,
                    address,
                    size,
                })
                .await
            {
                if !is_selected {
                    return;
                }

                bus.do_send(EnclaveEvent::from(CiphernodeSelected {
                    e3_id: data.e3_id,
                    nodecount: data.nodecount,
                }));
            }
        })
    }
}
