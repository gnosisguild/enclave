// TODO: spawn and supervise child actors
use crate::{
    CiphernodeOrchestrator, CommitteeRequested, E3id, EnclaveEvent, EventBus, Fhe,
    PlaintextOrchestrator, PublicKeyOrchestrator, Subscribe,
};
use actix::prelude::*;
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

pub struct OrchestratorBuilder {
    bus: Addr<EventBus>,
    rng: Arc<Mutex<ChaCha20Rng>>,
    public_key: Option<Addr<PublicKeyOrchestrator>>,
    plaintext: Option<Addr<PlaintextOrchestrator>>,
    ciphernode: Option<Addr<CiphernodeOrchestrator>>,
}

impl OrchestratorBuilder {
    pub fn new(bus: Addr<EventBus>, rng: Arc<Mutex<ChaCha20Rng>>) -> Self {
        Self {
            bus,
            rng,
            public_key: None,
            plaintext: None,
            ciphernode: None,
        }
    }

    pub fn public_key(mut self, value: Addr<PublicKeyOrchestrator>) -> Self {
        self.public_key = Some(value);
        self
    }

    pub fn plaintext(mut self, value: Addr<PlaintextOrchestrator>) -> Self {
        self.plaintext = Some(value);
        self
    }

    pub fn ciphernode(mut self, value: Addr<CiphernodeOrchestrator>) -> Self {
        self.ciphernode = Some(value);
        self
    }

    pub async fn build(self) -> Addr<Orchestrator> {
        let bus = self.bus;
        let rng = self.rng;
        let public_key = self.public_key;
        let plaintext = self.plaintext;
        let ciphernode = self.ciphernode;
        Orchestrator::attach(bus, rng, public_key, plaintext, ciphernode).await
    }
}

pub struct Orchestrator {
    fhes: HashMap<E3id, Addr<Fhe>>,
    meta: HashMap<E3id, CommitteeMeta>,
    public_key: Option<Addr<PublicKeyOrchestrator>>,
    plaintext: Option<Addr<PlaintextOrchestrator>>,
    ciphernode: Option<Addr<CiphernodeOrchestrator>>,
    rng: Arc<Mutex<ChaCha20Rng>>,
}

impl Orchestrator {
    pub fn builder(bus: Addr<EventBus>, rng: Arc<Mutex<ChaCha20Rng>>) -> OrchestratorBuilder {
        OrchestratorBuilder::new(bus, rng)
    }

    // TODO: use a builder pattern to manage the Option<Orchestrator>
    pub async fn attach(
        bus: Addr<EventBus>,
        rng: Arc<Mutex<ChaCha20Rng>>,
        public_key: Option<Addr<PublicKeyOrchestrator>>,
        plaintext: Option<Addr<PlaintextOrchestrator>>,
        ciphernode: Option<Addr<CiphernodeOrchestrator>>,
    ) -> Addr<Self> {
        let addr = Orchestrator {
            rng,
            public_key,
            plaintext,
            ciphernode,
            meta: HashMap::new(),
            fhes: HashMap::new(),
        }
        .start();
        bus.send(Subscribe::new("*", addr.clone().into()))
            .await
            .unwrap();
        addr
    }
}

impl Actor for Orchestrator {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for Orchestrator {
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

impl Orchestrator {
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
