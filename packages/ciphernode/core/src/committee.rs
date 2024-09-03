use std::collections::HashMap;

use actix::{Actor, Addr, Context, Handler, Message};

use crate::{
    committee_key::{CommitteeKey},
    decryption::Decryption,
    eventbus::EventBus,
    events::{E3id, EnclaveEvent},
    fhe::Fhe,
    Subscribe,
};

#[derive(Message)]
#[rtype(result = "()")]
pub struct Die;

struct CommitteeMeta {
    nodecount: usize,
}
pub struct CommitteeManager {
    bus: Addr<EventBus>,
    fhe: Addr<Fhe>,

    keys: HashMap<E3id, Addr<CommitteeKey>>,
    decryptions: HashMap<E3id, Addr<Decryption>>,
    meta: HashMap<E3id, CommitteeMeta>,
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
            decryptions: HashMap::new(),
            meta: HashMap::new(),
        }
    }

    pub fn attach(bus: Addr<EventBus>, fhe: Addr<Fhe>) -> Addr<Self> {
        let addr = CommitteeManager::new(bus.clone(), fhe).start();
        bus.do_send(Subscribe::new(
            "CommitteeRequested",
            addr.clone().recipient(),
        ));
        bus.do_send(Subscribe::new("KeyshareCreated", addr.clone().into()));
        bus.do_send(Subscribe::new("DecryptionRequested", addr.clone().into()));
        bus.do_send(Subscribe::new("DecryptionshareCreated", addr.clone().into()));
        bus.do_send(Subscribe::new("DecryptionOutputPublished", addr.clone().into()));
        addr
    }
}

impl Handler<EnclaveEvent> for CommitteeManager {
    type Result = ();

    fn handle(&mut self, event: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        match event {
            EnclaveEvent::CommitteeRequested { data, .. } => {
                // start up a new key
                let key = CommitteeKey::new(
                    self.fhe.clone(),
                    self.bus.clone(),
                    data.e3_id.clone(),
                    data.nodecount,
                )
                .start();

                self.meta.insert(
                    data.e3_id.clone(),
                    CommitteeMeta {
                        nodecount: data.nodecount.clone(),
                    },
                );
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
            }
            EnclaveEvent::DecryptionRequested { data, .. } => {
                let Some(meta) = self.meta.get(&data.e3_id) else {
                    // TODO: setup proper logger / telemetry
                    println!("E3Id not found in committee");
                    return;
                };
                // start up a new key
                let key = Decryption::new(
                    self.fhe.clone(),
                    self.bus.clone(),
                    data.e3_id.clone(),
                    meta.nodecount.clone(),
                )
                .start();

                self.decryptions.insert(data.e3_id, key);
            }
            EnclaveEvent::DecryptionshareCreated { data, .. } => {
                if let Some(decryption) = self.decryptions.get(&data.e3_id) {
                    decryption.do_send(data);
                }
            }
            EnclaveEvent::DecryptedOutputPublished { data, .. } => {
                let Some(addr) = self.decryptions.get(&data.e3_id) else {
                    return;
                };

                addr.do_send(Die);
                self.decryptions.remove(&data.e3_id);
            }
        }
    }
}
