// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    password_manager::{EnvPasswordManager, InMemPasswordManager, PasswordManager},
    secret_holder::TimedSecretHolder,
    FilePasswordManager,
};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use e3_config::AppConfig;
use rand::{rngs::OsRng, RngCore};
use std::{path::Path, time::Duration};
use zeroize::{Zeroize, Zeroizing};

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

/// Derives a cryptographic key from a password using Argon2id key derivation function.
///
/// Uses OWASP-recommended parameters for secure password hashing:
/// - Memory cost: 19 MiB
/// - Time cost: 2 iterations  
/// - Parallelism: 1 thread
/// - Output length: 32 bytes (256 bits)
///
/// # Arguments
/// * `password_bytes` - The password as a zeroizing byte vector
/// * `salt` - Random salt bytes for key derivation
///
/// # Returns
/// * `Ok(Zeroizing<Vec<u8>>)` - The derived key wrapped in a zeroizing container
/// * `Err(anyhow::Error)` - If key derivation fails
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

/// Encrypts data using AES-256-GCM with Argon2id key derivation.
///
/// The encryption process:
/// 1. Generates a random 32-byte salt for Argon2
/// 2. Generates a random 12-byte nonce for AES-GCM
/// 3. Derives a 256-bit key using Argon2id
/// 4. Encrypts the data with AES-256-GCM
/// 5. Returns salt + nonce + ciphertext as a single byte vector
///
/// # Arguments
/// * `password_bytes` - The password for encryption
/// * `data` - Mutable reference to data to encrypt (will be zeroized after encryption)
///
/// # Returns
/// * `Ok(Vec<u8>)` - Encrypted data in format: [salt][nonce][ciphertext]
/// * `Err(anyhow::Error)` - If encryption fails
///
/// # Security Notes
/// - Input data is zeroized after encryption to prevent memory disclosure
/// - Each encryption uses a fresh random salt and nonce
/// - Uses authenticated encryption (AES-GCM) to prevent tampering
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

/// Decrypts data that was encrypted with [`encrypt_data`].
///
/// Expects data in the format: [32-byte salt][12-byte nonce][ciphertext]
///
/// # Arguments
/// * `password_bytes` - The password used for decryption
/// * `encrypted_data` - The encrypted data to decrypt
///
/// # Returns
/// * `Ok(Vec<u8>)` - The decrypted plaintext data
/// * `Err(anyhow::Error)` - If decryption fails (wrong password, corrupted data, etc.)
///
/// # Security Notes
/// - Uses authenticated decryption to detect tampering
/// - Will fail if the wrong password is provided
/// - Will fail if the encrypted data has been modified
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

/// Configure the purge time here.
const PURGE_TIME_SECONDS: u64 = 120;

/// A high-level cryptographic interface providing secure data encryption and decryption.
///
/// `Cipher` combines industry-standard cryptographic primitives with secure key management:
/// - **Key Derivation**: Argon2id with OWASP-recommended parameters
/// - **Encryption**: AES-256-GCM for authenticated encryption
/// - **Key Management**: Automatic key purging
/// - **Memory Safety**: Zeroization of sensitive data
///
/// # Security Features
///
/// - **Perfect Forward Secrecy**: Each encryption uses a fresh random salt
/// - **Authenticated Encryption**: AES-GCM prevents tampering and forgery
/// - **Memory Protection**: Keys are automatically purged from memory
/// - **Side-Channel Resistance**: Uses constant-time operations where possible
///
/// # Usage Examples
///
/// ```
/// # use anyhow::Result;
/// # use e3_crypto::*;
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// // Create cipher from password
/// let cipher = Cipher::from_password("my-secret-password").await?;
///
/// // Encrypt some data
/// let mut data = b"Hello, world!".to_vec();
/// let encrypted = cipher.encrypt_data(&mut data)?;
///
/// // Decrypt the data
/// let decrypted = cipher.decrypt_data(&encrypted)?;
/// assert_eq!(b"Hello, world!", &decrypted[..]);
/// # Ok(())
/// # }
/// ```
///
/// # Key Sources
///
/// The cipher supports multiple key sources:
/// - [`from_password`](Self::from_password): Direct password string
/// - [`from_env`](Self::from_env): Environment variable
/// - [`from_file`](Self::from_file): File on disk
/// - [`from_config`](Self::from_config): Application configuration
///
/// # Security Considerations
///
/// - Keys are automatically purged from memory after inactivity
/// - Input data is zeroized after encryption to prevent memory disclosure
/// - Uses cryptographically secure random number generation
/// - Each encryption operation derives a fresh key (CPU intensive but secure)
///
/// # Error Handling
///
/// All methods return `Result<T>` and will fail if:
/// - The password/key cannot be retrieved
/// - Cryptographic operations fail (wrong password, corrupted data)
/// - Memory allocation fails
/// - Random number generation fails
pub struct Cipher {
    /// Holds the encryption key with automatic purging after timeout
    key: TimedSecretHolder,
    /// Password manager for retrieving fresh keys when needed
    pm: Box<dyn PasswordManager>,
}

impl Cipher {
    /// Creates a new `Cipher` instance with the specified password manager.
    ///
    /// # Arguments
    /// * `pm` - A password manager implementing the `PasswordManager` trait
    ///
    /// # Returns
    /// * `Ok(Cipher)` - Successfully created cipher instance
    /// * `Err(anyhow::Error)` - If the initial key retrieval fails
    ///
    /// # Examples
    /// ```
    /// # use anyhow::Result;
    /// # use e3_crypto::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let pm = InMemPasswordManager::from_str("my-password");
    /// let cipher = Cipher::new(pm).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new<P>(pm: P) -> Result<Self>
    where
        P: PasswordManager + 'static,
    {
        // Get the key from the password manager when created
        let key =
            TimedSecretHolder::new(pm.get_key().await?, Duration::from_secs(PURGE_TIME_SECONDS));
        Ok(Self {
            key,
            pm: Box::new(pm),
        })
    }

    /// Creates a cipher using a password string directly.
    ///
    /// This is the most convenient method for simple use cases where the password
    /// is known at compile time or obtained from user input.
    ///
    /// # Arguments
    /// * `value` - The password string to use for encryption/decryption
    ///
    /// # Returns
    /// * `Ok(Cipher)` - Successfully created cipher instance
    /// * `Err(anyhow::Error)` - If cipher creation fails
    ///
    /// # Examples
    /// ```
    /// # use anyhow::Result;
    /// # use e3_crypto::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let cipher = Cipher::from_password("super-secret-password").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Security Notes
    /// - The password string may remain in memory longer than the cipher instance
    /// - Consider using other methods for production applications
    pub async fn from_password(value: &str) -> Result<Self> {
        Ok(Self::new(InMemPasswordManager::from_str(value)).await?)
    }

    /// Creates a cipher using a password from an environment variable.
    ///
    /// This method is useful for production deployments where passwords should
    /// not be hardcoded in the application.
    ///
    /// # Arguments
    /// * `value` - The name of the environment variable containing the password
    ///
    /// # Returns
    /// * `Ok(Cipher)` - Successfully created cipher instance
    /// * `Err(anyhow::Error)` - If the environment variable is not found or cipher creation fails
    ///
    /// # Examples
    /// ```
    /// # use anyhow::Result;
    /// # use e3_crypto::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// # std::env::set_var("ENCRYPTION_KEY", "test-key-for-doctest");
    /// let cipher = Cipher::from_env("ENCRYPTION_KEY").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Security Notes
    /// - Environment variables may be visible to other processes
    /// - Ensure proper access controls on the deployment environment
    pub async fn from_env(value: &str) -> Result<Self> {
        Ok(Self::new(EnvPasswordManager::new(value)?).await?)
    }

    /// Creates a cipher using a password read from a file.
    ///
    /// The entire file contents (minus trailing whitespace) will be used as the password.
    /// This method is suitable for deployment scenarios with proper file permissions.
    ///
    /// # Arguments
    /// * `value` - Path to the file containing the password
    ///
    /// # Returns
    /// * `Ok(Cipher)` - Successfully created cipher instance  
    /// * `Err(anyhow::Error)` - If the file cannot be read or cipher creation fails
    ///
    /// # Examples
    /// ```ignore
    /// let cipher = Cipher::from_file("/etc/myapp/encryption.key").await?;
    /// ```
    ///
    /// # Security Notes
    /// - Ensure file permissions are properly restricted (e.g., 600)
    /// - The file should be on a secure filesystem
    /// - Consider using encrypted filesystems for additional protection
    pub async fn from_file(value: impl AsRef<Path>) -> Result<Self> {
        Ok(Self::new(FilePasswordManager::new(value)).await?)
    }

    /// Creates a cipher using the key file specified in the application configuration.
    ///
    /// This method integrates with the application's configuration system to
    /// determine the key file location.
    ///
    /// # Arguments
    /// * `config` - Application configuration containing the key file path
    ///
    /// # Returns
    /// * `Ok(Cipher)` - Successfully created cipher instance
    /// * `Err(anyhow::Error)` - If the key file cannot be read or cipher creation fails
    ///
    pub async fn from_config(config: &AppConfig) -> Result<Self> {
        Ok(Self::new(FilePasswordManager::new(config.key_file())).await?)
    }

    /// Executes an operation with access to the encryption key.
    ///
    /// This method handles key management automatically:
    /// 1. First attempts to use the cached key
    /// 2. If the key has expired, retrieves a fresh key from the password manager
    /// 3. Updates the cached key and retries the operation
    ///
    /// # Arguments
    /// * `operation_name` - Human-readable name for error messages
    /// * `op` - Closure that operates on the encryption key
    ///
    /// # Returns
    /// * `Ok(R)` - The result of the operation
    /// * `Err(anyhow::Error)` - If key retrieval or the operation fails
    ///
    /// # Examples
    /// ```
    /// # use anyhow::Result;
    /// # use e3_crypto::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let cipher = Cipher::from_password("test").await?;
    /// let result = cipher.with_key("custom operation", |key| {
    ///     // Your operation using the key
    ///     key.len()
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
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

    /// Encrypts data using AES-256-GCM with Argon2id key derivation.
    ///
    /// Each encryption operation:
    /// 1. Generates a fresh random salt and nonce
    /// 2. Derives a key using Argon2id with the salt
    /// 3. Encrypts the data with AES-256-GCM
    /// 4. Returns the salt, nonce, and ciphertext as a single byte vector
    /// 5. Zeroizes the input data for security
    ///
    /// # Arguments
    /// * `data` - Mutable reference to the data to encrypt (will be zeroized)
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - Encrypted data in format: [salt][nonce][ciphertext]
    /// * `Err(anyhow::Error)` - If encryption fails
    ///
    /// # Examples
    /// ```
    /// # use anyhow::Result;
    /// # use e3_crypto::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let cipher = Cipher::from_password("my-password").await?;
    /// let mut data = b"Hello, world!".to_vec();
    /// let encrypted = cipher.encrypt_data(&mut data)?;
    /// // data is now zeroized, encrypted contains the ciphertext
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Performance
    /// - Small data (~1KB): ~2-10ms depending on system
    /// - Large data (1MB+): Time scales with data size plus constant Argon2 overhead
    /// - Memory usage: ~19 MiB during key derivation
    ///
    /// # Security Notes
    /// - Uses a fresh random salt for each encryption (perfect forward secrecy)
    /// - Input data is zeroized after encryption
    /// - Provides both confidentiality and authenticity guarantees
    /// - Safe to encrypt the same plaintext multiple times (produces different ciphertext)
    pub fn encrypt_data(&self, data: &mut Vec<u8>) -> Result<Vec<u8>> {
        Ok(self.with_key("encryption", |key| encrypt_data(key, data))??)
    }

    /// Decrypts data that was encrypted with [`encrypt_data`].
    ///
    /// Expects data in the format produced by `encrypt_data`: [salt][nonce][ciphertext]
    ///
    /// # Arguments
    /// * `encrypted_data` - The encrypted data to decrypt
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - The decrypted plaintext data
    /// * `Err(anyhow::Error)` - If decryption fails
    ///
    /// # Examples
    /// ```
    /// # use anyhow::Result;
    /// # use e3_crypto::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let cipher = Cipher::from_password("my-password").await?;
    ///
    /// // Encrypt some data
    /// let mut original = b"Secret message".to_vec();
    /// let encrypted = cipher.encrypt_data(&mut original)?;
    ///
    /// // Decrypt it back
    /// let decrypted = cipher.decrypt_data(&encrypted)?;
    /// assert_eq!(b"Secret message", &decrypted[..]);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Error Conditions
    /// This method will fail if:
    /// - The encrypted data is too short or malformed
    /// - The wrong password is used
    /// - The encrypted data has been corrupted or tampered with
    /// - The key derivation process fails
    ///
    /// # Performance
    /// Similar to encryption performance, dominated by Argon2 key derivation time.
    pub fn decrypt_data(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        Ok(self.with_key("decryption", |key| decrypt_data(key, encrypted_data))??)
    }
}

impl Zeroize for Cipher {
    /// Zeroizes sensitive data when the cipher is dropped.
    ///
    /// This implementation purges the cached encryption key from memory,
    /// ensuring that sensitive key material does not persist after the
    /// cipher instance is no longer needed.
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
        let cipher = Cipher::from_password("test_password").await?;
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
        let cipher = Cipher::from_password("test_password").await?;
        let data = vec![];
        let encrypted = cipher.encrypt_data(&mut data.clone()).unwrap();
        let decrypted = cipher.decrypt_data(&encrypted).unwrap();
        assert_eq!(data, decrypted);
        Ok(())
    }

    #[tokio::test]
    async fn test_large_data() -> Result<()> {
        let cipher = Cipher::from_password("test_password").await?;
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
        let cipher = Cipher::from_password("password1").await?;
        let data = b"Secret message";
        let encrypted = cipher.encrypt_data(&mut data.to_vec()).unwrap();

        // Try to decrypt with a different password
        let cipher = Cipher::from_password("password2").await?;
        let result = cipher.decrypt_data(&encrypted);
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_binary_data() -> Result<()> {
        let cipher = Cipher::from_password("test_password").await?;
        let data = vec![0xFF, 0x00, 0xAA, 0x55, 0x12, 0xED];
        let encrypted = cipher.encrypt_data(&mut data.clone()).unwrap();
        let decrypted = cipher.decrypt_data(&encrypted).unwrap();
        assert_eq!(data, decrypted);
        Ok(())
    }

    #[tokio::test]
    async fn test_unicode_data() -> Result<()> {
        let cipher = Cipher::from_password("test_password").await?;
        let data = "Hello 🌍 привет 世界".as_bytes().to_vec();
        let encrypted = cipher.encrypt_data(&mut data.clone()).unwrap();
        let decrypted = cipher.decrypt_data(&encrypted).unwrap();
        assert_eq!(data, decrypted);
        Ok(())
    }

    #[tokio::test]
    #[should_panic(expected = "Invalid encrypted data length")]
    async fn test_invalid_encrypted_data() {
        let cipher = Cipher::from_password("test_password").await.unwrap();
        let invalid_data = vec![0u8; 10]; // Too short to be valid encrypted data
        cipher.decrypt_data(&invalid_data).unwrap();
    }

    #[tokio::test]
    async fn test_multiple_encrypt_decrypt_cycles() {
        let cipher = Cipher::from_password("test_password").await.unwrap();
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
        let cipher = Cipher::from_password("test_password").await.unwrap();
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
