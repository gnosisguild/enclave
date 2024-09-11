use std::collections::HashMap;

use crate::{ciphernode_sequencer::CiphernodeSequencer, plaintext_sequencer::PlaintextSequencer, publickey_sequencer::PublicKeySequencer, E3id};

struct Registry {
    pub public_keys: HashMap<E3id, PublicKeySequencer>,
    pub plaintexts: HashMap<E3id, PlaintextSequencer>,
    pub ciphernodes: HashMap<E3id, CiphernodeSequencer>
}
