// TODO: spawn and supervise child actors
// TODO: vertically modularize this so there is a registry for each function that get rolled up into one based
// on config
use crate::{
    CiphernodeSequencer, CommitteeRequested, Data, E3id, EnclaveEvent, EventBus, Fhe,
    PlaintextSequencer, PublicKeySequencer, Sortition, Subscribe,
};
use actix::prelude::*;
use alloy_primitives::Address;
use rand_chacha::ChaCha20Rng;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
struct CommitteeMeta {
    nodecount: usize,
}

pub struct Registry {
    bus: Addr<EventBus>,
    ciphernodes: HashMap<E3id, Addr<CiphernodeSequencer>>,
    data: Addr<Data>,
    sortition: Addr<Sortition>,
    address: Address,
    fhes: HashMap<E3id, Addr<Fhe>>,
    plaintexts: HashMap<E3id, Addr<PlaintextSequencer>>,
    meta: HashMap<E3id, CommitteeMeta>,
    public_keys: HashMap<E3id, Addr<PublicKeySequencer>>,
    rng: Arc<Mutex<ChaCha20Rng>>,
}

impl Registry {
    pub fn new(
        bus: Addr<EventBus>,
        data: Addr<Data>,
        sortition: Addr<Sortition>,
        rng: Arc<Mutex<ChaCha20Rng>>,
        address: Address,
    ) -> Self {
        Self {
            bus,
            data,
            sortition,
            rng,
            address,
            ciphernodes: HashMap::new(),
            plaintexts: HashMap::new(),
            public_keys: HashMap::new(),
            meta: HashMap::new(),
            fhes: HashMap::new(),
        }
    }

    pub async fn attach(
        bus: Addr<EventBus>,
        data: Addr<Data>,
        sortition: Addr<Sortition>,
        rng: Arc<Mutex<ChaCha20Rng>>,
        address: Address,
    ) -> Addr<Self> {
        let addr = Registry::new(bus.clone(), data, sortition, rng, address).start();
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
                    sortition_seed,
                    ..
                } = data;

                let fhe_factory = self.fhe_factory(moduli, degree, plaintext_modulus, crp);
                let fhe = store(&e3_id, &mut self.fhes, fhe_factory);
                let meta = CommitteeMeta {
                    nodecount: data.nodecount,
                };

                self.meta.entry(e3_id.clone()).or_insert(meta.clone());

                let public_key_sequencer_factory =
                    self.public_key_sequencer_factory(e3_id.clone(), meta.clone(), fhe.clone(), sortition_seed);
                store(&e3_id, &mut self.public_keys, public_key_sequencer_factory);

                let ciphernode_sequencer_factory = self.ciphernode_sequencer_factory(fhe.clone());
                store(&e3_id, &mut self.ciphernodes, ciphernode_sequencer_factory);
            }
            EnclaveEvent::CiphertextOutputPublished { .. } => {
                let Some(fhe) = self.fhes.get(&e3_id) else {
                    return;
                };

                let Some(meta) = self.meta.get(&e3_id) else {
                    return;
                };

                let plaintext_sequencer_factory =
                    self.plaintext_sequencer_factory(e3_id.clone(), meta.clone(), fhe.clone());
                store(&e3_id, &mut self.plaintexts, plaintext_sequencer_factory);
            }
            _ => (),
        };

        self.forward_message(&e3_id, msg);
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

    fn public_key_sequencer_factory(
        &self,
        e3_id: E3id,
        meta: CommitteeMeta,
        fhe: Addr<Fhe>,
        seed: u64,
    ) -> impl FnOnce() -> Addr<PublicKeySequencer> {
        let bus = self.bus.clone();
        let nodecount = meta.nodecount;
        let sortition = self.sortition.clone();
        move || PublicKeySequencer::new(fhe, e3_id, sortition, bus, nodecount, seed).start()
    }

    fn ciphernode_sequencer_factory(
        &self,
        fhe: Addr<Fhe>,
    ) -> impl FnOnce() -> Addr<CiphernodeSequencer> {
        let data = self.data.clone();
        let bus = self.bus.clone();
        let address = self.address;
        move || CiphernodeSequencer::new(fhe, data, bus, address).start()
    }

    fn plaintext_sequencer_factory(
        &self,
        e3_id: E3id,
        meta: CommitteeMeta,
        fhe: Addr<Fhe>,
    ) -> impl FnOnce() -> Addr<PlaintextSequencer> {
        let bus = self.bus.clone();
        let nodecount = meta.nodecount;
        move || PlaintextSequencer::new(fhe, e3_id, bus, nodecount).start()
    }

    fn forward_message(&self, e3_id: &E3id, msg: EnclaveEvent) {
        if let Some(act) = self.public_keys.get(e3_id) {
            act.clone().recipient().do_send(msg.clone());
        }

        if let Some(act) = self.plaintexts.get(e3_id) {
            act.do_send(msg.clone());
        }

        if let Some(act) = self.ciphernodes.get(e3_id) {
            act.do_send(msg.clone());
        }
    }
}

// Store on a hashmap a Addr<T> from the factory F
fn store<T, F>(e3_id: &E3id, map: &mut HashMap<E3id, Addr<T>>, creator: F) -> Addr<T>
where
    T: Actor<Context = Context<T>>,
    F: FnOnce() -> Addr<T>,
{
    map.entry(e3_id.clone()).or_insert_with(creator).clone()
}
