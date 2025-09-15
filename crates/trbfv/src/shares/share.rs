// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_crypto::{Cipher, SensitiveBytes, ToSensitiveBytes};
use std::ops::Deref;

use crate::shares::pvw::PvwShare;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Share(pub Vec<u64>);

impl Deref for Share {
    type Target = Vec<u64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Share {
    pub fn new(v: Vec<u64>) -> Self {
        Self(v)
    }

    pub fn into_vec(self) -> Vec<u64> {
        self.0
    }

    // This currently serializes but will eventually encrypt to pvw
    // Expect to have keys passed in here
    pub fn try_into_pvw(self) -> Result<PvwShare> {
        Ok(PvwShare::new(bincode::serialize(&self.0)?))
    }

    // This currently deserializes but will eventually decrypt to pvw
    // Expect to have keys passed in here
    pub fn try_from_pvw(value: PvwShare) -> Result<Share> {
        Ok(bincode::deserialize(&value.as_bytes())?)
    }
}

impl ToSensitiveBytes for Share {
    fn encrypt(&self, cipher: &Cipher) -> Result<SensitiveBytes> {
        Ok(SensitiveBytes::new(bincode::serialize(&self.0)?, cipher)?)
    }
}
