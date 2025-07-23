// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::hex;
use alloy_primitives::Uint;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Seed(pub [u8; 32]);
impl From<Seed> for u64 {
    fn from(value: Seed) -> Self {
        u64::from_le_bytes(value.0[..8].try_into().unwrap())
    }
}

impl From<Seed> for [u8; 32] {
    fn from(value: Seed) -> Self {
        value.0
    }
}

impl From<Uint<256, 4>> for Seed {
    fn from(value: Uint<256, 4>) -> Self {
        Seed(value.to_le_bytes())
    }
}

impl Display for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Seed(0x{})", hex::encode(self.0))
    }
}
