// TODO: spawn and supervise child actors
// TODO: vertically modularize this so there is a registry for each function that get rolled up into one based
// on config
use crate::{
    Ciphernode, CiphernodeRegistry, CommitteeRequested, Data, E3id, EnclaveEvent, EventBus, Fhe,
    PlaintextAggregator, PlaintextRegistry, PublicKeyAggregator, PublicKeyRegistry, Sortition,
    Subscribe,
};
use actix::prelude::*;
use alloy_primitives::Address;
use fhe::mbfv::PublicKeySwitchShare;
use rand_chacha::ChaCha20Rng;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct InitializeWithEnclaveEvent {
    pub fhe: Addr<Fhe>,
    pub meta: CommitteeMeta,
    pub event: EnclaveEvent,
}

impl InitializeWithEnclaveEvent {
    pub fn new(fhe: Addr<Fhe>, meta: CommitteeMeta, event: EnclaveEvent) -> Self {
        Self { fhe, meta, event }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitteeMeta {
    pub nodecount: usize,
    pub seed: u64,
}

pub struct Registry {
    bus: Addr<EventBus>,
    fhes: HashMap<E3id, Addr<Fhe>>,
    meta: HashMap<E3id, CommitteeMeta>,
    public_key: Option<Addr<PublicKeyRegistry>>,
    plaintext: Option<Addr<PlaintextRegistry>>,
    ciphernode: Option<Addr<CiphernodeRegistry>>,
    rng: Arc<Mutex<ChaCha20Rng>>,
}

impl Registry {
    pub fn new(
        bus: Addr<EventBus>,
        rng: Arc<Mutex<ChaCha20Rng>>,
        public_key: Option<Addr<PublicKeyRegistry>>,
        plaintext: Option<Addr<PlaintextRegistry>>,
        ciphernode: Option<Addr<CiphernodeRegistry>>,
    ) -> Self {
        Self {
            bus,
            rng,
            public_key,
            plaintext,
            ciphernode,
            meta: HashMap::new(),
            fhes: HashMap::new(),
        }
    }

    pub async fn attach(
        bus: Addr<EventBus>,
        rng: Arc<Mutex<ChaCha20Rng>>,
        public_key: Option<Addr<PublicKeyRegistry>>,
        plaintext: Option<Addr<PlaintextRegistry>>,
        ciphernode: Option<Addr<CiphernodeRegistry>>,
    ) -> Addr<Self> {
        let addr = Registry::new(bus.clone(), rng, public_key, plaintext, ciphernode).start();
        bus.send(Subscribe::new("*", addr.clone().into()))
            .await
            .unwrap();
        addr
    }
}

impl Actor for Registry {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for Registry {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        let Some(e3_id) = msg.get_e3_id() else {
            return;
        };

        match msg.clone() {
            EnclaveEvent::CommitteeRequested { data, .. } => {
                let CommitteeRequested {
                    degree,
                    moduli,
                    plaintext_modulus,
                    crp,
                    ..
                } = data;

                let fhe_factory = self.fhe_factory(moduli, degree, plaintext_modulus, crp);
                let fhe = self.fhes.entry(e3_id.clone()).or_insert_with(fhe_factory);
                let meta = CommitteeMeta {
                    nodecount: data.nodecount,
                    seed: data.sortition_seed,
                };
                self.meta.entry(e3_id.clone()).or_insert(meta.clone());

                if let Some(addr) = self.public_key.clone() {
                    addr.do_send(InitializeWithEnclaveEvent {
                        event: msg.clone(),
                        fhe: fhe.clone(),
                        meta: meta.clone(),
                    })
                }

                if let Some(addr) = self.ciphernode.clone() {
                    addr.do_send(InitializeWithEnclaveEvent {
                        event: msg.clone(),
                        fhe: fhe.clone(),
                        meta: meta.clone(),
                    })
                }
            }
            EnclaveEvent::CiphertextOutputPublished { data, .. } => {
                let Some(plaintext) = self.plaintext.clone() else {
                    return;
                };

                let Some(fhe) = self.fhes.get(&data.e3_id) else {
                    return;
                };

                let Some(meta) = self.meta.get(&data.e3_id) else {
                    return;
                };

                plaintext.do_send(InitializeWithEnclaveEvent {
                    event: msg.clone(),
                    fhe: fhe.clone(),
                    meta: meta.clone(),
                })
            }
            _ => (),
        };

        self.forward_message(msg);
    }
}

impl Registry {
    fn fhe_factory(
        &self,
        moduli: Vec<u64>,
        degree: usize,
        plaintext_modulus: u64,
        crp: Vec<u8>,
    ) -> impl FnOnce() -> Addr<Fhe> {
        let rng = self.rng.clone();
        move || {
            Fhe::from_raw_params(&moduli, degree, plaintext_modulus, &crp, rng)
                .unwrap()
                .start()
        }
    }

    fn forward_message(&mut self, msg: EnclaveEvent) {
        if let Some(addr) = self.ciphernode.clone() {
            addr.do_send(msg.clone())
        }

        if let Some(addr) = self.public_key.clone() {
            addr.do_send(msg.clone())
        }

        if let Some(addr) = self.plaintext.clone() {
            addr.do_send(msg.clone())
        }
    }
}
