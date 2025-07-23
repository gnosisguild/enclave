// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt,
    hash::{DefaultHasher, Hash, Hasher},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub [u8; 32]);

impl EventId {
    pub fn hash<T: Hash>(value: T) -> Self {
        let mut hasher = Sha256::new();
        let mut std_hasher = DefaultHasher::new();
        value.hash(&mut std_hasher);
        hasher.update(std_hasher.finish().to_le_bytes());
        let result = hasher.finalize();
        EventId(result.into())
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let base58_string = bs58::encode(&self.0).into_string();
        write!(f, "evt:{}", &base58_string[0..8])
    }
}
