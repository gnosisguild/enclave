use std::collections::HashMap;

use actix::{Actor, Addr, Context, Handler};

use crate::{
    committee_key::{CommitteeKey, Die},
    eventbus::EventBus,
    events::{E3id, EnclaveEvent},
    fhe::Fhe,
};

pub struct CommitteeManager {
    bus: Addr<EventBus>,
    fhe: Addr<Fhe>,
    aggregators: HashMap<E3id, Addr<CommitteeKey>>,
}

impl Actor for CommitteeManager {
    type Context = Context<Self>;
}

impl CommitteeManager {
    pub fn new(bus: Addr<EventBus>, fhe: Addr<Fhe>) -> Self {
        Self {
            bus,
            fhe,
            aggregators: HashMap::new(),
        }
    }
}

impl Handler<EnclaveEvent> for CommitteeManager {
    type Result = ();

    fn handle(&mut self, event: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        match event {
            EnclaveEvent::ComputationRequested { data, .. } => {
                // start up a new aggregator
                let aggregator = CommitteeKey::new(
                    self.fhe.clone(),
                    self.bus.clone(),
                    data.e3_id.clone(),
                    data.nodecount,
                )
                .start();

                self.aggregators.insert(data.e3_id, aggregator);
            }
            EnclaveEvent::KeyshareCreated { data, .. } => {
                if let Some(aggregator) = self.aggregators.get(&data.e3_id) {
                    aggregator.do_send(data);
                }
            },
            EnclaveEvent::PublicKeyAggregated { data, .. } => {
                let Some(aggregator) = self.aggregators.get(&data.e3_id) else {
                    return;
                };

                aggregator.do_send(Die);
                self.aggregators.remove(&data.e3_id);
            }
            // _ => (),
        }
    }
}
