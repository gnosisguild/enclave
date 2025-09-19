// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use anyhow::Result;

use super::DeserializableValue;

/// Encrypted version of T for secure storage/transmission
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
// TODO: Currently we are simply serializing the data in preparation for PVW encryption
// expecting to pass in keys to encrypt and decrypt as required at a later point
pub struct PvwEncrypted<T>(Vec<u8>, std::marker::PhantomData<T>);

impl<T> PvwEncrypted<T>
where
    T: DeserializableValue,
{
    /// Create a new encrypted wrapper from data of type T
    pub fn new(data: T) -> Result<Self> {
        Ok(Self(bincode::serialize(&data)?, std::marker::PhantomData))
    }

    /// Decrypt and deserialize back to type T
    pub fn pvw_decrypt(self) -> Result<T> {
        let value = self.0;
        Ok(bincode::deserialize(&value)?)
    }
}

pub trait PvwEncryptedVecExt<T> {
    fn to_vec_decrypted(self) -> Result<Vec<T>>;
}

impl<T> PvwEncryptedVecExt<T> for Vec<PvwEncrypted<T>>
where
    T: DeserializableValue,
{
    fn to_vec_decrypted(self) -> Result<Vec<T>> {
        self.into_iter().map(|s| s.pvw_decrypt()).collect()
    }
}

/// Trait to add decrypt functionality to Vec<Encrypted<T>>
pub trait PvwEncryptableVec<T> {
    /// Decrypt all encrypted values in the vector
    /// Returns a vector of decrypted values or the first error encountered
    fn pvw_encrypt(self) -> Result<Vec<PvwEncrypted<T>>>;
}

impl<T> PvwEncryptableVec<T> for Vec<T>
where
    T: DeserializableValue,
{
    fn pvw_encrypt(self) -> Result<Vec<PvwEncrypted<T>>> {
        self.into_iter()
            .map(|encryptable| PvwEncrypted::new(encryptable))
            .collect()
    }
}
