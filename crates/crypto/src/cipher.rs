// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{path::Path, time::Duration};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use e3_config::AppConfig;
use rand::{rngs::OsRng, RngCore};
use zeroize::{Zeroize, Zeroizing};

use crate::{
    password_manager::{EnvPasswordManager, InMemPasswordManager, PasswordManager},
    secret_holder::TimedSecretHolder,
    FilePasswordManager,
};

// ARGON2 PARAMS
// https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
const ARGON2_M_COST: u32 = 19 * 1024; // 19 MiB
const ARGON2_T_COST: u32 = 2;
const ARGON2_P_COST: u32 = 1;
const ARGON2_OUTPUT_LEN: usize = 32;
const ARGON2_ALGORITHM: Algorithm = Algorithm::Argon2id;
const ARGON2_VERSION: Version = Version::V0x13;

// AES PARAMS
const AES_SALT_LEN: usize = 32;
const AES_NONCE_LEN: usize = 12;

fn argon2_derive_key(
    password_bytes: &Zeroizing<Vec<u8>>,
    salt: &[u8],
) -> Result<Zeroizing<Vec<u8>>> {
    let mut derived_key = Zeroizing::new(vec![0u8; ARGON2_OUTPUT_LEN]);

    let params = Params::new(
        ARGON2_M_COST,
        ARGON2_T_COST,
        ARGON2_P_COST,
        Some(ARGON2_OUTPUT_LEN),
    )
    .map_err(|_| anyhow!("Could not create params"))?;
    Argon2::new(ARGON2_ALGORITHM, ARGON2_VERSION, params)
        .hash_password_into(&password_bytes, &salt, &mut derived_key)
        .map_err(|_| anyhow!("Key derivation error"))?;
    Ok(derived_key)
}

fn encrypt_data(password_bytes: &Zeroizing<Vec<u8>>, data: &mut Vec<u8>) -> Result<Vec<u8>> {
    // Generate a random salt for Argon2
    let mut salt = [0u8; AES_SALT_LEN];
    OsRng.fill_bytes(&mut salt);

    // Generate a random nonce for AES-GCM
    let mut nonce_bytes = [0u8; AES_NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Derive key using Argon2
    let derived_key = argon2_derive_key(password_bytes, &salt)?;

    // Create AES-GCM cipher
    let cipher = Aes256Gcm::new_from_slice(&derived_key)?;

    // Encrypt the data
    let ciphertext = cipher
        .encrypt(nonce, data.as_ref())
        .map_err(|_| anyhow!("Could not AES Encrypt given plaintext."))?;

    data.zeroize(); // Zeroize sensitive input data

    // Pack data
    let mut output = Vec::with_capacity(salt.len() + nonce_bytes.len() + ciphertext.len());
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);

    Ok(output)
}

fn decrypt_data(password_bytes: &Zeroizing<Vec<u8>>, encrypted_data: &[u8]) -> Result<Vec<u8>> {
    const AES_HEADER_LEN: usize = AES_SALT_LEN + AES_NONCE_LEN;
    if encrypted_data.len() < AES_HEADER_LEN {
        return Err(anyhow!("Invalid encrypted data length"));
    }

    // Extract salt and nonce
    let salt = &encrypted_data[..AES_SALT_LEN];
    let nonce = Nonce::from_slice(&encrypted_data[AES_SALT_LEN..AES_HEADER_LEN]);
    let ciphertext = &encrypted_data[AES_HEADER_LEN..];

    // Derive key using Argon2
    let derived_key = argon2_derive_key(password_bytes, &salt)?;

    // Create cipher and decrypt
    let cipher = Aes256Gcm::new_from_slice(&derived_key)?;
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow!("Could not decrypt data"))?;

    Ok(plaintext)
}

pub struct Cipher {
    key: TimedSecretHolder,
    pm: Box<dyn PasswordManager>,
}

impl Cipher {
    pub fn new<P>(pm: P) -> Result<Self>
    where
        P: PasswordManager + 'static,
    {
        // Get the key from the password manager when created
        let key = TimedSecretHolder::new(pm.get_key_sync()?, Duration::from_secs(1));
        Ok(Self {
            key,
            pm: Box::new(pm),
        })
    }

    pub fn from_password(value: &str) -> Result<Self> {
        Ok(Self::new(InMemPasswordManager::from_str(value))?)
    }

    pub fn from_env(value: &str) -> Result<Self> {
        Ok(Self::new(EnvPasswordManager::new(value)?)?)
    }

    pub fn from_file(value: impl AsRef<Path>) -> Result<Self> {
        Ok(Self::new(FilePasswordManager::new(value))?)
    }

    pub fn from_config(config: &AppConfig) -> Result<Self> {
        Ok(Self::new(FilePasswordManager::new(config.key_file()))?)
    }

    /// Run an operation with the local
    pub fn with_key<F, R>(&self, operation_name: &str, mut op: F) -> Result<R>
    where
        F: FnMut(&Zeroizing<Vec<u8>>) -> R,
    {
        // Try with current key first
        if let Some(result) = self.key.access(|key| op(key)) {
            return Ok(result);
        }

        // Didn't work so get the key again
        let fresh_key = self.pm.get_key_sync()?;

        // Update the key on the container
        self.key.update(fresh_key);

        // Run the operation again
        Ok(self
            .key
            .access(|key| op(key))
            .ok_or_else(|| anyhow!("Could not complete {}: key update failed", operation_name))?)
    }

    pub fn encrypt_data(&self, data: &mut Vec<u8>) -> Result<Vec<u8>> {
        Ok(self.with_key("encryption", |key| encrypt_data(key, data))??)
    }

    pub fn decrypt_data(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        Ok(self.with_key("decryption", |key| decrypt_data(key, encrypted_data))??)
    }
}

impl Zeroize for Cipher {
    fn zeroize(&mut self) {
        self.key.purge();
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_basic_encryption_decryption() -> Result<()> {
        let data = b"Hello, world!";

        let start = Instant::now();

        let cipher = Cipher::from_password("test_password")?;
        let encrypted = cipher.encrypt_data(&mut data.to_vec()).unwrap();
        let encryption_time = start.elapsed();

        let start = Instant::now();
        let decrypted = cipher.decrypt_data(&encrypted).unwrap();
        let decryption_time = start.elapsed();

        println!("Encryption took: {:?}", encryption_time);
        println!("Decryption took: {:?}", decryption_time);
        println!("Total time: {:?}", encryption_time + decryption_time);

        assert_eq!(data, &decrypted[..]);
        Ok(())
    }

    #[tokio::test]
    async fn test_empty_data() -> Result<()> {
        let cipher = Cipher::from_password("test_password")?;
        let data = vec![];

        let encrypted = cipher.encrypt_data(&mut data.clone()).unwrap();
        let decrypted = cipher.decrypt_data(&encrypted).unwrap();

        assert_eq!(data, decrypted);
        Ok(())
    }

    #[tokio::test]
    async fn test_large_data() -> Result<()> {
        let cipher = Cipher::from_password("test_password")?;
        let data = vec![1u8; 1024 * 1024]; // 1MB of data

        let start = Instant::now();
        let encrypted = cipher.encrypt_data(&mut data.clone()).unwrap();
        let encryption_time = start.elapsed();

        let start = Instant::now();
        let decrypted = cipher.decrypt_data(&encrypted).unwrap();
        let decryption_time = start.elapsed();

        println!("Large data encryption took: {:?}", encryption_time);
        println!("Large data decryption took: {:?}", decryption_time);

        assert_eq!(data, decrypted);
        Ok(())
    }

    #[tokio::test]
    async fn test_different_passwords() -> Result<()> {
        // Encrypt with one password
        let cipher = Cipher::from_password("password1")?;

        let data = b"Secret message";
        let encrypted = cipher.encrypt_data(&mut data.to_vec()).unwrap();

        // Try to decrypt with a different password
        let cipher = Cipher::from_password("password2")?;
        let result = cipher.decrypt_data(&encrypted);

        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_binary_data() -> Result<()> {
        let cipher = Cipher::from_password("test_password")?;

        let data = vec![0xFF, 0x00, 0xAA, 0x55, 0x12, 0xED];

        let encrypted = cipher.encrypt_data(&mut data.clone()).unwrap();
        let decrypted = cipher.decrypt_data(&encrypted).unwrap();

        assert_eq!(data, decrypted);
        Ok(())
    }

    #[tokio::test]
    async fn test_unicode_data() -> Result<()> {
        let cipher = Cipher::from_password("test_password")?;
        let data = "Hello 🌍 привет 世界".as_bytes().to_vec();

        let encrypted = cipher.encrypt_data(&mut data.clone()).unwrap();
        let decrypted = cipher.decrypt_data(&encrypted).unwrap();

        assert_eq!(data, decrypted);
        Ok(())
    }

    #[tokio::test]
    #[should_panic(expected = "Invalid encrypted data length")]
    async fn test_invalid_encrypted_data() {
        let cipher = Cipher::from_password("test_password").unwrap();
        let invalid_data = vec![0u8; 10]; // Too short to be valid encrypted data
        cipher.decrypt_data(&invalid_data).unwrap();
    }

    #[tokio::test]
    async fn test_multiple_encrypt_decrypt_cycles() {
        let cipher = Cipher::from_password("test_password").unwrap();
        let original_data = b"Multiple encryption cycles test";

        let mut data = original_data.to_vec();
        for _ in 0..5 {
            data = cipher.encrypt_data(&mut data).unwrap();
            data = cipher.decrypt_data(&data).unwrap();
        }

        assert_eq!(original_data.to_vec(), data);
    }

    #[tokio::test]
    async fn test_corrupted_data() {
        let cipher = Cipher::from_password("test_password").unwrap();
        let data = b"Test corrupted data";

        let mut encrypted = cipher.encrypt_data(&mut data.to_vec()).unwrap();

        // Corrupt the ciphertext portion (after salt and nonce)
        if let Some(byte) = encrypted.get_mut(AES_SALT_LEN + AES_NONCE_LEN) {
            *byte ^= 0xFF;
        }

        let result = cipher.decrypt_data(&encrypted);
        assert!(result.is_err());
    }
}
