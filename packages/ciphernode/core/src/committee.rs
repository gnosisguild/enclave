use std::collections::HashMap;

use actix::{Actor, Addr, Context, Handler};

use crate::{
    committee_key::{CommitteeKey, Die},
    eventbus::EventBus,
    events::{E3id, EnclaveEvent},
    fhe::Fhe,
    Subscribe,
};

pub struct CommitteeManager {
    bus: Addr<EventBus>,
    fhe: Addr<Fhe>,
    keys: HashMap<E3id, Addr<CommitteeKey>>,
}

impl Actor for CommitteeManager {
    type Context = Context<Self>;
}

impl CommitteeManager {
    pub fn new(bus: Addr<EventBus>, fhe: Addr<Fhe>) -> Self {
        Self {
            bus,
            fhe,
            keys: HashMap::new(),
        }
    }

    pub fn attach(bus: Addr<EventBus>, fhe: Addr<Fhe>) -> Addr<Self> {
        let addr = CommitteeManager::new(bus.clone(), fhe).start();
        bus.do_send(Subscribe::new(
            "ComputationRequested",
            addr.clone().recipient(),
        ));
        bus.do_send(Subscribe::new("KeyshareCreated", addr.clone().into()));
        addr
    }
}

impl Handler<EnclaveEvent> for CommitteeManager {
    type Result = ();

    fn handle(&mut self, event: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        match event {
            EnclaveEvent::ComputationRequested { data, .. } => {
                // start up a new key
                let key = CommitteeKey::new(
                    self.fhe.clone(),
                    self.bus.clone(),
                    data.e3_id.clone(),
                    data.nodecount,
                )
                .start();

                self.keys.insert(data.e3_id, key);
            }
            EnclaveEvent::KeyshareCreated { data, .. } => {
                if let Some(key) = self.keys.get(&data.e3_id) {
                    key.do_send(data);
                }
            }
            EnclaveEvent::PublicKeyAggregated { data, .. } => {
                let Some(key) = self.keys.get(&data.e3_id) else {
                    return;
                };

                key.do_send(Die);
                self.keys.remove(&data.e3_id);
            } // _ => (),
        }
    }
}
