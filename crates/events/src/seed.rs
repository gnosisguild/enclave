// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::hex;
use alloy_primitives::Uint;
use derivative::Derivative;
use e3_utils::{AsBytesSerde, BytesSerde};
use std::fmt::{self, Display};

#[derive(Derivative, BytesSerde, Clone, Copy, PartialEq, Eq, Hash)]
#[derivative(Debug)]
pub struct Seed(#[derivative(Debug(format_with = "e3_utils::formatters::hexf"))] pub [u8; 32]);
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

impl AsBytesSerde for Seed {
    fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        Ok(Seed(
            bytes.try_into().map_err(|_| "EventId requires 32 bytes")?,
        ))
    }
}

impl Display for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Seed(0x{})", hex::encode(self.0))
    }
}
