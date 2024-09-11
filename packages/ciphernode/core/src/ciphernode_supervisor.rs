use crate::{
    eventbus::EventBus,
    events::{E3id, EnclaveEvent},
    fhe::Fhe,
    plaintext_aggregator::PlaintextAggregator,
    publickey_aggregator::PublicKeyAggregator,
    Subscribe,
};
use actix::prelude::*;
use std::collections::HashMap;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Die;

// CommitteeMeta
// Storing metadata around the committee eg threshold / nodecount
struct CommitteeMeta {
    nodecount: usize,
}

pub struct CiphernodeSupervisor {
    bus: Addr<EventBus>,
    fhe: Addr<Fhe>,

    publickey_aggregators: HashMap<E3id, Addr<PublicKeyAggregator>>,
    plaintext_aggregators: HashMap<E3id, Addr<PlaintextAggregator>>,
    meta: HashMap<E3id, CommitteeMeta>,
}

impl Actor for CiphernodeSupervisor {
    type Context = Context<Self>;
}

impl CiphernodeSupervisor {
    pub fn new(bus: Addr<EventBus>, fhe: Addr<Fhe>) -> Self {
        Self {
            bus,
            fhe,
            publickey_aggregators: HashMap::new(),
            plaintext_aggregators: HashMap::new(),
            meta: HashMap::new(),
        }
    }

    pub fn attach(bus: Addr<EventBus>, fhe: Addr<Fhe>) -> Addr<Self> {
        let addr = CiphernodeSupervisor::new(bus.clone(), fhe).start();
        bus.do_send(Subscribe::new(
            "CommitteeRequested",
            addr.clone().recipient(),
        ));
        bus.do_send(Subscribe::new("KeyshareCreated", addr.clone().into()));
        bus.do_send(Subscribe::new(
            "CiphertextOutputPublished",
            addr.clone().into(),
        ));
        bus.do_send(Subscribe::new(
            "DecryptionshareCreated",
            addr.clone().into(),
        ));
        bus.do_send(Subscribe::new("PlaintextAggregated", addr.clone().into()));
        addr
    }
}

impl Handler<EnclaveEvent> for CiphernodeSupervisor {
    type Result = ();

    fn handle(&mut self, event: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        match event {
            EnclaveEvent::CommitteeRequested { data, .. } => {
                // start up a new key
                let publickey_aggregator = PublicKeyAggregator::new(
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
                self.publickey_aggregators
                    .insert(data.e3_id, publickey_aggregator);
            }
            EnclaveEvent::KeyshareCreated { data, .. } => {
                if let Some(key) = self.publickey_aggregators.get(&data.e3_id) {
                    key.do_send(data);
                }
            }
            EnclaveEvent::PublicKeyAggregated { data, .. } => {
                let Some(publickey_aggregator) = self.publickey_aggregators.get(&data.e3_id) else {
                    return;
                };

                publickey_aggregator.do_send(Die);
                self.publickey_aggregators.remove(&data.e3_id);
            }
            EnclaveEvent::CiphertextOutputPublished { data, .. } => {
                let Some(meta) = self.meta.get(&data.e3_id) else {
                    // TODO: setup proper logger / telemetry
                    println!("E3Id not found in committee");
                    return;
                };
                // start up a new key
                let plaintext_aggregator = PlaintextAggregator::new(
                    self.fhe.clone(),
                    self.bus.clone(),
                    data.e3_id.clone(),
                    meta.nodecount.clone(),
                )
                .start();

                self.plaintext_aggregators
                    .insert(data.e3_id, plaintext_aggregator);
            }
            EnclaveEvent::DecryptionshareCreated { data, .. } => {
                if let Some(decryption) = self.plaintext_aggregators.get(&data.e3_id) {
                    decryption.do_send(EnclaveEvent::from(data));
                }
            }
            EnclaveEvent::PlaintextAggregated { data, .. } => {
                let Some(addr) = self.plaintext_aggregators.get(&data.e3_id) else {
                    return;
                };

                addr.do_send(Die);
                self.plaintext_aggregators.remove(&data.e3_id);
            },
            _ => ()
        }
    }
}
