// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_crypto::{Cipher, SensitiveBytes};

use crate::shares::share::Share;
use crate::shares::share_set::ShareSet;
use crate::shares::share_set_collection::ShareSetCollection;

/// Encrypted version of ShareSetCollection for secure storage/transmission
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EncryptedShareSetCollection(Vec<Vec<SensitiveBytes>>);

impl EncryptedShareSetCollection {
    pub fn new(data: Vec<Vec<SensitiveBytes>>) -> Self {
        Self(data)
    }

    pub fn decrypt(&self, cipher: &Cipher) -> Result<ShareSetCollection> {
        let out = self
            .0
            .iter()
            .map(|v| {
                Ok(ShareSet(
                    v.iter()
                        .map(|s| Ok(Share::new(bincode::deserialize(&s.access_raw(cipher)?)?)))
                        .collect::<Result<Vec<Share>>>()?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(ShareSetCollection(out))
    }
}
