// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cid(pub Vec<u8>);

impl Cid {
    pub fn from_content(content: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hashed = hasher.finalize();
        Self(hashed.to_vec())
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}

impl Deref for Cid {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl DerefMut for Cid {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl AsRef<[u8]> for Cid {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl ToString for Cid {
    fn to_string(&self) -> String {
        hex::encode(&self.0)
    }
}
