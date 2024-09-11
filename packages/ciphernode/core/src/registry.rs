use std::collections::HashMap;

use actix::prelude::*;

use crate::{CiphernodeSequencer, E3id, EnclaveEvent, Fhe, PlaintextSequencer, PublicKeySequencer};

pub struct Registry {
    pub public_keys: HashMap<E3id, Addr<PublicKeySequencer>>,
    pub plaintexts: HashMap<E3id, Addr<PlaintextSequencer>>,
    pub ciphernodes: HashMap<E3id, Addr<CiphernodeSequencer>>,
    pub fhes: HashMap<E3id, Addr<Fhe>>
}

impl Actor for Registry {
    type Context = Context<Self>;
}
//
// impl Handler<EnclaveEvent> for Registry {
//     type Result = ();
//     fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
//         let e3_id = E3id::from(msg.clone());
//         match msg {
//             EnclaveEvent::CommitteeRequested { data, .. } => {
//
//              let moduli =  &vec![0x3FFFFFFF000001];
//             let degree = 2048;
//             let plaintext_modulus = 1032193;
//
//
//                 self.fhes.entry(e3_id).or_insert_with(|| Fhe::(
//                self.public_keys.entry(e3_id).or_insert_with(|| PublicKeySequencer::new()); 
//             },
//             _ => ()
//         }
//     }
// }
