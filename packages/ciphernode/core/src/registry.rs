use std::collections::HashMap;

use actix::prelude::*;
use rand_chacha::ChaCha20Rng;

use crate::{
    CiphernodeSequencer, Data, E3id, EnclaveEvent, EventBus, Fhe, PlaintextSequencer,
    PublicKeySequencer,
};

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
        let mut fhes = self.fhes.clone();
        let bus = self.bus.clone();
        let mut public_keys = self.public_keys.clone();
        let mut plaintexts = self.plaintexts.clone();
        let mut ciphernodes = self.ciphernodes.clone();
        let data = self.data.clone();

        // Idempotently create references
        // TODO: this adds coupling here would be nice to be more abstract
        match msg.clone() {
            EnclaveEvent::CommitteeRequested { .. } => {
                let moduli = &vec![0x3FFFFFFF000001];
                let degree = 2048;
                let plaintext_modulus = 1032193;

                let fhe = fhes.entry(e3_id.clone()).or_insert_with(|| {
                    Fhe::from_raw_params(moduli, degree, plaintext_modulus, self.rng.clone())
                        .unwrap()
                        .start()
                });

                public_keys.entry(e3_id.clone()).or_insert_with(|| {
                    PublicKeySequencer::new(fhe.clone(), e3_id.clone(), bus.clone(), self.nodecount)
                        .start()
                });

                ciphernodes.entry(e3_id.clone()).or_insert_with(|| {
                    CiphernodeSequencer::new(fhe.clone(), data.clone(), bus.clone()).start()
                });
            }
            EnclaveEvent::CiphertextOutputPublished { .. } => {
                let fhe = fhes.get(&e3_id).unwrap();
                plaintexts.entry(e3_id.clone()).or_insert_with(|| {
                    PlaintextSequencer::new(fhe.clone(), e3_id.clone(), bus, self.nodecount).start()
                });
            }
            _ => (),
        };

        // Can I iterate over each of these?
        if let Some(act) = public_keys.get(&e3_id) {
            act.do_send(msg.clone());
        }

        if let Some(act) = plaintexts.get(&e3_id) {
            act.do_send(msg.clone());
        }

        if let Some(act) = ciphernodes.get(&e3_id) {
            act.do_send(msg.clone());
        }
    }
}
