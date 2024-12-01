use anyhow::*;
use cipher::Cipher;
use libp2p::identity::Keypair;
use serde::{Deserialize, Serialize};

/// Hold an encrypted libp2p keypair in a serializable way for storage
#[derive(Clone, Serialize, Deserialize)]
pub struct EncryptedKeypair {
    secret: Vec<u8>,
}

impl EncryptedKeypair {
    /// Generate an encrypted Keypair and store it in memory encrypted to the given Cipher
    pub fn generate(cipher: &Cipher) -> Result<Self> {
        let mut secret_raw = Keypair::generate_ed25519()
            .try_into_ed25519()?
            .to_bytes()
            .to_vec();

        let secret = cipher.encrypt_data(&mut secret_raw)?;
        Ok(Self { secret })
    }

    /// Decrypt the Keypair with the given cipher and return it
    pub fn decrypt(&self, cipher: &Cipher) -> Result<Keypair> {
        Ok(Keypair::ed25519_from_bytes(
            cipher.decrypt_data(&self.secret)?,
        )?)
    }
}
