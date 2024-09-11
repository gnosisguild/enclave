use crate::{
    CiphernodeSequencer, Data, E3id, EnclaveEvent, EventBus, Fhe, PlaintextSequencer,
    PublicKeySequencer,
};
use actix::prelude::*;
use rand_chacha::ChaCha20Rng;
use std::collections::HashMap;

pub struct Registry {
    bus: Addr<EventBus>,
    ciphernodes: HashMap<E3id, Addr<CiphernodeSequencer>>,
    data: Addr<Data>,
    fhes: HashMap<E3id, Addr<Fhe>>,
    nodecount: usize,
    plaintexts: HashMap<E3id, Addr<PlaintextSequencer>>,
    public_keys: HashMap<E3id, Addr<PublicKeySequencer>>,
    rng: ChaCha20Rng,
}

impl Actor for Registry {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for Registry {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        let e3_id = E3id::from(msg.clone());

        match msg.clone() {
            EnclaveEvent::CommitteeRequested { .. } => {
                let fhe_creator = self.fhe_creator();
                let fhe = store(&e3_id, &mut self.fhes, fhe_creator);

                let public_key_sequencer_factory = self.public_key_sequencer_factory(e3_id.clone(), fhe.clone());
                store(&e3_id, &mut self.public_keys, public_key_sequencer_factory);

                let ciphernode_sequencer_factory = self.ciphernode_sequencer_factory(fhe.clone());
                store(&e3_id, &mut self.ciphernodes, ciphernode_sequencer_factory);
            }
            EnclaveEvent::CiphertextOutputPublished { .. } => {
                let Some(fhe) = self.fhes.get(&e3_id) else {
                    return;
                };
                let plaintext_sequencer_factory = self.plaintext_sequencer_factory(e3_id.clone(), fhe.clone());
                store(&e3_id, &mut self.plaintexts, plaintext_sequencer_factory);
            }
            _ => (),
        };

        self.forward_message(&e3_id, msg);
    }
}

impl Registry {
    fn fhe_creator(&self) -> impl FnOnce() -> Addr<Fhe> {
        let rng = self.rng.clone();
        move || {
            let moduli = &vec![0x3FFFFFFF000001];
            let degree = 2048;
            let plaintext_modulus = 1032193;
            Fhe::from_raw_params(moduli, degree, plaintext_modulus, rng)
                .unwrap()
                .start()
        }
    }

    fn public_key_sequencer_factory(
        &self,
        e3_id: E3id,
        fhe: Addr<Fhe>,
    ) -> impl FnOnce() -> Addr<PublicKeySequencer> {
        let bus = self.bus.clone();
        let nodecount = self.nodecount;
        move || PublicKeySequencer::new(fhe, e3_id, bus, nodecount).start()
    }

    fn ciphernode_sequencer_factory(&self, fhe: Addr<Fhe>) -> impl FnOnce() -> Addr<CiphernodeSequencer> {
        let data = self.data.clone();
        let bus = self.bus.clone();
        move || CiphernodeSequencer::new(fhe, data, bus).start()
    }

    fn plaintext_sequencer_factory(
        &self,
        e3_id: E3id,
        fhe: Addr<Fhe>,
    ) -> impl FnOnce() -> Addr<PlaintextSequencer> {
        let bus = self.bus.clone();
        let nodecount = self.nodecount;
        move || PlaintextSequencer::new(fhe, e3_id, bus, nodecount).start()
    }

    fn forward_message(&self, e3_id: &E3id, msg: EnclaveEvent) {
        if let Some(act) = self.public_keys.get(&e3_id) {
            act.clone().recipient().do_send(msg.clone());
        }

        if let Some(act) = self.plaintexts.get(&e3_id) {
            act.do_send(msg.clone());
        }

        if let Some(act) = self.ciphernodes.get(&e3_id) {
            act.do_send(msg.clone());
        }
    }
}

fn store<T, F>(e3_id: &E3id, map: &mut HashMap<E3id, Addr<T>>, creator: F) -> Addr<T>
where
    T: Actor<Context = Context<T>>,
    F: FnOnce() -> Addr<T>,
{
    map.entry(e3_id.clone()).or_insert_with(creator).clone()
}
