use std::{env, ops::Deref};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::{rngs::OsRng, RngCore};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

// ARGON2 PARAMS
const ARGON2_M_COST: u32 = 32 * 1024; // 32 MiB
const ARGON2_T_COST: u32 = 2;
const ARGON2_P_COST: u32 = 2;
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

    // NOTE: password_bytes and derived_key will be automatically zeroized when dropped

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

    // NOTE: password_bytes and derived_key will be automatically zeroized when dropped

    Ok(plaintext)
}

#[derive(ZeroizeOnDrop)]
pub struct Encryptor {
    key: Zeroizing<Vec<u8>>,
}

impl Encryptor {
    pub fn new(mut secret: String) -> Self {
        let key = Zeroizing::new(secret.as_bytes().to_vec());
        secret.zeroize();
        Self { key }
    }

    pub fn from_env(key:&str) -> Result<Self> {
        Ok(Self::new(env::var(key)?))
    }

    pub fn encrypt_data(&self, data: &mut Vec<u8>) -> Result<Vec<u8>> {
        encrypt_data(&self.key, data)
    }

    pub fn decrypt_data(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        decrypt_data(&self.key, encrypted_data)
    }
}

impl Zeroize for Encryptor {
    fn zeroize(&mut self) {
        self.key.zeroize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_basic_encryption_decryption() {
        let data = b"Hello, world!";

        let start = Instant::now();
        let encryptor = Encryptor::new("my_secure_password".to_owned());
        let encrypted = encryptor.encrypt_data(&mut data.to_vec()).unwrap();
        let encryption_time = start.elapsed();

        let start = Instant::now();
        let decrypted = encryptor.decrypt_data(&encrypted).unwrap();
        let decryption_time = start.elapsed();

        println!("Encryption took: {:?}", encryption_time);
        println!("Decryption took: {:?}", decryption_time);
        println!("Total time: {:?}", encryption_time + decryption_time);

        assert_eq!(data, &decrypted[..]);
    }

    #[test]
    fn test_empty_data() {
        let encryptor = Encryptor::new("test_password".to_owned());

        let data = vec![];

        let encrypted = encryptor.encrypt_data(&mut data.clone()).unwrap();
        let decrypted = encryptor.decrypt_data(&encrypted).unwrap();

        assert_eq!(data, decrypted);
    }

    #[test]
    fn test_large_data() {
        let encryptor = Encryptor::new("test_password".to_owned());
        let data = vec![0u8; 1024 * 1024]; // 1MB of data

        let start = Instant::now();
        let encrypted = encryptor.encrypt_data(&mut data.clone()).unwrap();
        let encryption_time = start.elapsed();

        let start = Instant::now();
        let decrypted = encryptor.decrypt_data(&encrypted).unwrap();
        let decryption_time = start.elapsed();

        println!("Large data encryption took: {:?}", encryption_time);
        println!("Large data decryption took: {:?}", decryption_time);

        assert_eq!(data, decrypted);
    }

    #[test]
    fn test_different_passwords() {
        // Encrypt with one password
        let encryptor = Encryptor::new("password1".to_owned());

        let data = b"Secret message";
        let encrypted = encryptor.encrypt_data(&mut data.to_vec()).unwrap();

        // Try to decrypt with a different password
        let encryptor = Encryptor::new("password2".to_owned());
        let result = encryptor.decrypt_data(&encrypted);

        assert!(result.is_err());
    }

    #[test]
    fn test_binary_data() {
        let encryptor = Encryptor::new("test_password".to_owned());

        let data = vec![0xFF, 0x00, 0xAA, 0x55, 0x12, 0xED];

        let encrypted = encryptor.encrypt_data(&mut data.clone()).unwrap();
        let decrypted = encryptor.decrypt_data(&encrypted).unwrap();

        assert_eq!(data, decrypted);
    }

    #[test]
    fn test_unicode_data() {
        let encryptor = Encryptor::new("test_password".to_owned());
        let data = "Hello 🌍 привет 世界".as_bytes().to_vec();

        let encrypted = encryptor.encrypt_data(&mut data.clone()).unwrap();
        let decrypted = encryptor.decrypt_data(&encrypted).unwrap();

        assert_eq!(data, decrypted);
    }

    #[test]
    #[should_panic(expected = "Invalid encrypted data length")]
    fn test_invalid_encrypted_data() {
        let encryptor = Encryptor::new("test_password".to_owned());
        let invalid_data = vec![0u8; 10]; // Too short to be valid encrypted data
        encryptor.decrypt_data(&invalid_data).unwrap();
    }

    #[test]
    fn test_multiple_encrypt_decrypt_cycles() {
        let encryptor = Encryptor::new("test_password".to_owned());
        let original_data = b"Multiple encryption cycles test";

        let mut data = original_data.to_vec();
        for _ in 0..5 {
            data = encryptor.encrypt_data(&mut data).unwrap();
            data = encryptor.decrypt_data(&data).unwrap();
        }

        assert_eq!(original_data.to_vec(), data);
    }

    #[test]
    fn test_corrupted_data() {
        let encryptor = Encryptor::new("test_password".to_owned());
        let data = b"Test corrupted data";

        let mut encrypted = encryptor.encrypt_data(&mut data.to_vec()).unwrap();

        // Corrupt the ciphertext portion (after salt and nonce)
        if let Some(byte) = encrypted.get_mut(AES_SALT_LEN + AES_NONCE_LEN) {
            *byte ^= 0xFF;
        }

        let result = encryptor.decrypt_data(&encrypted);
        assert!(result.is_err());
    }
}
