use anyhow::*;
use cipher::Cipher;
use libp2p::identity::Keypair;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Hold an encrypted libp2p keypair in a serializable way for storage
#[derive(Clone, Serialize, Deserialize)]
pub struct EncryptedKeypair {
    secret: Vec<u8>,
}

impl EncryptedKeypair {
    fn generate_keypair() -> Keypair {
        Keypair::generate_ed25519()
    }

    fn to_secretkey(keypair: &Keypair) -> Result<Vec<u8>> {
        Ok(
            keypair.clone().try_into_ed25519()?.to_bytes()[..32] // first half is secret key
                .to_vec(),
        )
    }

    fn frombytes_ed25519(bytes: Vec<u8>) -> Result<Keypair> {
        Ok(Keypair::ed25519_from_bytes(bytes)?)
    }

    fn extract_encrypted(cipher: &Cipher, keypair: &Keypair) -> Result<Self> {
        let mut secret_raw = EncryptedKeypair::to_secretkey(keypair)?;
        let secret = cipher.encrypt_data(&mut secret_raw)?;
        Ok(Self { secret })
    }
    /// Generate an encrypted Keypair and store it in memory encrypted to the given Cipher
    pub fn generate(cipher: &Cipher) -> Result<Self> {
        let keypair = EncryptedKeypair::generate_keypair();
        warn!(
            "Generating peer id: {}",
            keypair.public().to_peer_id()
        );
        Ok(EncryptedKeypair::extract_encrypted(cipher, &keypair)?)
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
        let kp = EncryptedKeypair::generate_keypair();
        let secret_raw = EncryptedKeypair::to_secretkey(&kp)?;
        let kp2 = EncryptedKeypair::frombytes_ed25519(secret_raw)?;
        assert_eq!(kp.public(), kp2.public(), "Public keys should match");
        Ok(())
    }

    #[tokio::test]
    async fn generate_encrypted_without_error() -> Result<()> {
        let kp = EncryptedKeypair::generate_keypair();

        let cipher = Cipher::from_password("I am a secret phrase").await?;
        let encrypted = EncryptedKeypair::extract_encrypted(&cipher, &kp)?;
        let kp2 = encrypted.decrypt(&cipher)?;
        assert_eq!(
            kp.public(),
            kp2.public(),
            "Keypairs have the same public key"
        );

        Ok(())
    }
}
