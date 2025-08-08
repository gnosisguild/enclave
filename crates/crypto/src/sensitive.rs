// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::Cipher;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zeroize::Zeroizing;

/// A container that holds encrypted data
/// We could just use cipher to encrypt and decrypt bytes and pass that around but this
/// means we get the type system indicating when data is encrypted
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SensitiveBytes {
    encrypted: Arc<Vec<u8>>,
}

impl SensitiveBytes {
    /// Create a new Sensitive container by encrypting the provided data
    pub fn new(input: impl Into<Vec<u8>>, cipher: &Cipher) -> Result<Self> {
        let mut bytes = input.into();
        let encrypted = cipher.encrypt_data(&mut bytes)?;
        Ok(Self {
            encrypted: Arc::new(encrypted),
        })
    }

    /// Access the decrypted data, wrapped in a ZeroizeOnDrop container
    pub fn access(&self, cipher: &Cipher) -> Result<Zeroizing<Vec<u8>>> {
        let decrypted_data = cipher.decrypt_data(&self.encrypted)?;
        Ok(Zeroizing::new(decrypted_data))
    }
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
