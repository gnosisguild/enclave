use std::{env, ops::Deref};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::{rngs::OsRng, RngCore};
use zeroize::{Zeroize, Zeroizing};

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
    password_bytes: Zeroizing<Vec<u8>>,
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

pub fn encrypt_data(data: &mut Vec<u8>) -> Result<Vec<u8>> {
    // Convert password to bytes in a zeroizing buffer
    let password_bytes = Zeroizing::new(env::var("CIPHERNODE_SECRET")?.as_bytes().to_vec());

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

pub fn decrypt_data(encrypted_data: &[u8]) -> Result<Vec<u8>> {
    const AES_HEADER_LEN: usize = AES_SALT_LEN + AES_NONCE_LEN;
    if encrypted_data.len() < AES_HEADER_LEN {
        return Err(anyhow!("Invalid encrypted data length"));
    }

    let password_bytes = Zeroizing::new(env::var("CIPHERNODE_SECRET")?.as_bytes().to_vec());

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        use std::time::Instant;
        println!("TESTING");
        env::set_var("CIPHERNODE_SECRET", "my_secure_password");
        let data = b"Hello, world!";

        let start = Instant::now();
        let encrypted = encrypt_data(&mut data.to_vec()).unwrap();
        let encryption_time = start.elapsed();

        let start = Instant::now();
        let decrypted = decrypt_data(&encrypted).unwrap();
        let decryption_time = start.elapsed();

        println!("Encryption took: {:?}", encryption_time);
        println!("Decryption took: {:?}", decryption_time);
        println!("Total time: {:?}", encryption_time + decryption_time);

        assert_eq!(data, &decrypted[..]);
    }
}
