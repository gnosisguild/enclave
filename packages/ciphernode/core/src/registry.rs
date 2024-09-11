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
                self.create_committee_actors(&e3_id);
            }
            EnclaveEvent::CiphertextOutputPublished { .. } => {
                self.create_plaintext_actor(&e3_id);
            }
            _ => (),
        };

        self.forward_message(&e3_id, msg);
    }
}

impl Registry {
    fn create_committee_actors(&mut self, e3_id: &E3id) {
        let fhe = self.create_or_get_fhe(e3_id);
        self.create_or_get_public_key_sequencer(e3_id, fhe.clone());
        self.create_or_get_ciphernode_sequencer(e3_id, fhe);
    }

    fn create_or_get_fhe(&mut self, e3_id: &E3id) -> Addr<Fhe> {
        self.fhes
            .entry(e3_id.clone())
            .or_insert_with(|| {
                let moduli = &vec![0x3FFFFFFF000001];
                let degree = 2048;
                let plaintext_modulus = 1032193;
                Fhe::from_raw_params(moduli, degree, plaintext_modulus, self.rng.clone())
                    .unwrap()
                    .start()
            })
            .clone()
    }

    fn create_or_get_public_key_sequencer(&mut self, e3_id: &E3id, fhe: Addr<Fhe>) {
        self.public_keys.entry(e3_id.clone()).or_insert_with(|| {
            PublicKeySequencer::new(fhe, e3_id.clone(), self.bus.clone(), self.nodecount).start()
        });
    }

    fn create_or_get_ciphernode_sequencer(&mut self, e3_id: &E3id, fhe: Addr<Fhe>) {
        self.ciphernodes.entry(e3_id.clone()).or_insert_with(|| {
            CiphernodeSequencer::new(fhe, self.data.clone(), self.bus.clone()).start()
        });
    }

    fn create_plaintext_actor(&mut self, e3_id: &E3id) {
        if let Some(fhe) = self.fhes.get(e3_id) {
            self.plaintexts.entry(e3_id.clone()).or_insert_with(|| {
                PlaintextSequencer::new(
                    fhe.clone(),
                    e3_id.clone(),
                    self.bus.clone(),
                    self.nodecount,
                )
                .start()
            });
        }
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
