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
    fn generate_ed25519() -> Result<Vec<u8>> {
        Ok(
            Keypair::generate_ed25519().try_into_ed25519()?.to_bytes()[..32] // first half is secret key
                .to_vec(),
        )
    }

    fn frombytes_ed25519(bytes: Vec<u8>) -> Result<Keypair> {
        Ok(Keypair::ed25519_from_bytes(bytes)?)
    }

    /// Generate an encrypted Keypair and store it in memory encrypted to the given Cipher
    pub fn generate(cipher: &Cipher) -> Result<Self> {
        let mut secret_raw = EncryptedKeypair::generate_ed25519()?;
        let secret = cipher.encrypt_data(&mut secret_raw)?;
        Ok(Self { secret })
    }

    /// Decrypt the Keypair with the given cipher and return it
    pub fn decrypt(&self, cipher: &Cipher) -> Result<Keypair> {
        Ok(EncryptedKeypair::frombytes_ed25519(
            cipher.decrypt_data(&self.secret)?,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn generate_without_error() -> Result<()> {
        let bytes = EncryptedKeypair::generate_ed25519()?;
        EncryptedKeypair::frombytes_ed25519(bytes)?;
        Ok(())
    }

    #[tokio::test]
    async fn generate_encrypted_without_error() -> Result<()> {
        let cipher = Cipher::from_password("I am a secret phrase").await?;
        let encrypted = EncryptedKeypair::generate(&cipher)?;
        encrypted.decrypt(&cipher)?;

        Ok(())
    }
}
