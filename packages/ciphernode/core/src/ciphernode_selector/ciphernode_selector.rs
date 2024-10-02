use actix::prelude::*;
use crate::{enclave_core::{CiphernodeSelected, EnclaveEvent, EventBus, Subscribe}, sortition::{GetHasNode, Sortition}};

pub struct CiphernodeSelector {
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
    address: String,
}

impl Actor for CiphernodeSelector {
    type Context = Context<Self>;
}

impl CiphernodeSelector {
    pub fn new(bus: Addr<EventBus>, sortition: Addr<Sortition>, address: &str) -> Self {
        Self {
            bus,
            sortition,
            address: address.to_owned(),
        }
    }

    pub fn attach(bus: Addr<EventBus>, sortition: Addr<Sortition>, address: &str) -> Addr<Self> {
        let addr = CiphernodeSelector::new(bus.clone(), sortition, address).start();

        bus.do_send(Subscribe::new("E3Requested", addr.clone().recipient()));

        addr
    }
}

impl Handler<EnclaveEvent> for CiphernodeSelector {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, event: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        let address = self.address.clone();
        let sortition = self.sortition.clone();
        let bus = self.bus.clone();

        Box::pin(async move {
            let EnclaveEvent::E3Requested { data, .. } = event else {
                return;
            };

            let seed = data.seed;
            let size = data.threshold_m;

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
                    threshold_m: data.threshold_m,
                }));
            }
        })
    }
}
