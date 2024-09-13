// TODO: spawn and supervise child actors
// TODO: vertically modularize this so there is a registry for each function that get rolled up into one based
// on config
use crate::{
    Ciphernode, CommitteeRequested, Data, E3id, EnclaveEvent, EventBus, Fhe, PlaintextAggregator,
    PublicKeyAggregator, Sortition, Subscribe,
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
    seed: u64,
}

pub struct Registry {
    bus: Addr<EventBus>,
    ciphernodes: HashMap<E3id, Addr<Ciphernode>>,
    data: Addr<Data>,
    sortition: Addr<Sortition>,
    address: Address,
    fhes: HashMap<E3id, Addr<Fhe>>,
    plaintexts: HashMap<E3id, Addr<PlaintextAggregator>>,
    buffers: HashMap<E3id, HashMap<String, Vec<EnclaveEvent>>>,
    meta: HashMap<E3id, CommitteeMeta>,
    public_keys: HashMap<E3id, Addr<PublicKeyAggregator>>,
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
            buffers: HashMap::new(),
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
                    seed: data.sortition_seed,
                };

                self.meta.entry(e3_id.clone()).or_insert(meta.clone());

                let public_key_factory = self.public_key_factory(
                    e3_id.clone(),
                    meta.clone(),
                    fhe.clone(),
                    sortition_seed,
                );
                store(&e3_id, &mut self.public_keys, public_key_factory);

                let ciphernode_factory = self.ciphernode_factory(fhe.clone());
                store(&e3_id, &mut self.ciphernodes, ciphernode_factory);
            }
            EnclaveEvent::CiphertextOutputPublished { .. } => {
                let Some(fhe) = self.fhes.get(&e3_id) else {
                    return;
                };

                let Some(meta) = self.meta.get(&e3_id) else {
                    return;
                };

                let plaintext_factory =
                    self.plaintext_factory(e3_id.clone(), meta.clone(), fhe.clone());
                store(&e3_id, &mut self.plaintexts, plaintext_factory);
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

    fn public_key_factory(
        &self,
        e3_id: E3id,
        meta: CommitteeMeta,
        fhe: Addr<Fhe>,
        seed: u64,
    ) -> impl FnOnce() -> Addr<PublicKeyAggregator> {
        let bus = self.bus.clone();
        let nodecount = meta.nodecount;
        let sortition = self.sortition.clone();
        move || PublicKeyAggregator::new(fhe, bus, sortition, e3_id, nodecount, seed).start()
    }

    fn ciphernode_factory(&self, fhe: Addr<Fhe>) -> impl FnOnce() -> Addr<Ciphernode> {
        let data = self.data.clone();
        let bus = self.bus.clone();
        let address = self.address;
        move || Ciphernode::new(bus, fhe, data, address).start()
    }

    fn plaintext_factory(
        &self,
        e3_id: E3id,
        meta: CommitteeMeta,
        fhe: Addr<Fhe>,
    ) -> impl FnOnce() -> Addr<PlaintextAggregator> {
        let bus = self.bus.clone();
        let sortition = self.sortition.clone();
        let nodecount = meta.nodecount;
        let seed = meta.seed;
        move || PlaintextAggregator::new(fhe, bus, sortition, e3_id, nodecount, seed).start()
    }

    fn store_msg(&mut self, e3_id: E3id, msg: EnclaveEvent, key: &str) {
        self.buffers
            .entry(e3_id)
            .or_default()
            .entry(key.to_owned())
            .or_default()
            .push(msg);
    }

    fn take_msgs(&mut self, e3_id: E3id, key: &str) -> Vec<EnclaveEvent> {
        self.buffers
            .get_mut(&e3_id)
            .and_then(|inner_map| inner_map.get_mut(key))
            .map(std::mem::take)
            .unwrap_or_default()
    }

    fn forward_message(&mut self, e3_id: &E3id, msg: EnclaveEvent) {
        // Buffer events for each thing that has not been created
        // TODO: Needs tidying up as this is verbose and repeats
        // TODO: use an enum for the buffer keys
        if let Some(act) = self.public_keys.clone().get(e3_id) {
            let msgs = self.take_msgs(e3_id.clone(), "public_keys");
            let recipient = act.clone().recipient();
            recipient.do_send(msg.clone());
            for m in msgs {
                recipient.do_send(m);
            }
        } else {
            self.store_msg(e3_id.clone(), msg.clone(), "public_keys");
        }

        if let Some(act) = self.plaintexts.clone().get(e3_id) {
            let msgs = self.take_msgs(e3_id.clone(), "plaintexts");
            let recipient = act.clone().recipient();
            recipient.do_send(msg.clone());
            for m in msgs {
                recipient.do_send(m);
            }
        } else {
            self.store_msg(e3_id.clone(), msg.clone(), "plaintexts");
        }

        if let Some(act) = self.ciphernodes.clone().get(e3_id) {
            let msgs = self.take_msgs(e3_id.clone(), "ciphernodes");
            let recipient = act.clone().recipient();
            recipient.do_send(msg.clone());
            for m in msgs {
                recipient.do_send(m);
            }
        } else {
            self.store_msg(e3_id.clone(), msg.clone(), "ciphernodes");
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
