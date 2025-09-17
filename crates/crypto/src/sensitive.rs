// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::Cipher;
use anyhow::Result;
// use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zeroize::Zeroizing;

/// A container that holds encrypted data
/// We could just use cipher to encrypt and decrypt bytes and pass that around but this
/// means we get the type system indicating when data is encrypted
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SensitiveBytes {
    encrypted: Arc<Vec<u8>>,
}

impl SensitiveBytes {
    /// Create a new Sensitive container by encrypting the provided data
    // TODO: rename to try_new
    pub fn new(input: impl Into<Vec<u8>>, cipher: &Cipher) -> Result<Self> {
        let mut bytes = input.into();
        let encrypted = cipher.encrypt_data(&mut bytes)?;
        Ok(Self {
            encrypted: Arc::new(encrypted),
        })
    }

    /// Helper to access a vector of sensitive bytes
    // TODO: rename try_access_vec
    pub fn access_vec(
        sensitive_vec: Vec<SensitiveBytes>,
        cipher: &Cipher,
    ) -> Result<Vec<Zeroizing<Vec<u8>>>> {
        sensitive_vec
            .into_iter()
            .map(|s| s.access(cipher))
            .collect()
    }

    pub fn try_from_vec(inputs: Vec<Vec<u8>>, cipher: &Cipher) -> Result<Vec<Self>> {
        inputs
            .into_iter()
            .map(|i| SensitiveBytes::new(i, cipher))
            .collect::<Result<_>>()
    }

    pub fn try_from_unserialized_vec<T: Sized>(
        value: Vec<T>,
        cipher: &Cipher,
    ) -> Result<Vec<SensitiveBytes>>
    where
        T: ?Sized + serde::Serialize,
    {
        value
            .into_iter()
            .map(|s| SensitiveBytes::new(bincode::serialize(&s)?, cipher))
            .collect::<Result<_>>()
    }

    /// Access the decrypted data, wrapped in a ZeroizeOnDrop container
    // TODO: rename try_access
    pub fn access(&self, cipher: &Cipher) -> Result<Zeroizing<Vec<u8>>> {
        Ok(Zeroizing::new(self.access_raw(cipher)?))
    }

    pub fn access_raw(&self, cipher: &Cipher) -> Result<Vec<u8>> {
        cipher.decrypt_data(&self.encrypted)
    }
}

pub trait ToSensitiveBytes {
    fn encrypt(&self, cipher: &Cipher) -> Result<SensitiveBytes>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sensitive_basic_functionality() -> Result<()> {
        let cipher = Cipher::from_password("1243").await?;
        let original_data = b"Hello, World!".to_vec();
        let expected_data = original_data.clone();

        // Create sensitive container
        let sensitive = SensitiveBytes::new(original_data, &cipher).unwrap();

        // Access the data
        let accessed_data = sensitive.access(&cipher).unwrap();
        assert_eq!(accessed_data.as_slice(), expected_data);

        // Test cloning
        let cloned_sensitive = sensitive.clone();
        let cloned_accessed = cloned_sensitive.access(&cipher).unwrap();
        assert_eq!(cloned_accessed.as_slice(), expected_data);
        Ok(())
    }

    #[tokio::test]
    async fn test_sensitive_with_string() -> Result<()> {
        let cipher = Cipher::from_password("1243").await?;
        let original_string = "Secret message".to_string();
        let expected_bytes = original_string.clone();

        let sensitive = SensitiveBytes::new(original_string, &cipher).unwrap();
        let accessed_data = sensitive.access(&cipher).unwrap();

        assert_eq!(accessed_data.as_slice(), expected_bytes.as_bytes());
        Ok(())
    }
}
