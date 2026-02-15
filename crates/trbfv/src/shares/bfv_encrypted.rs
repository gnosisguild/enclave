// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{Context, Result};
use derivative::Derivative;
use e3_utils::utility_types::ArcBytes;
use fhe::bfv::{BfvParameters, Ciphertext, Encoding, Plaintext, PublicKey, SecretKey};
use fhe_traits::{
    DeserializeParametrized, FheDecoder, FheDecrypter, FheEncoder, FheEncrypter,
    Serialize as FheSerialize,
};
use ndarray::Array2;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::Arc;

use super::{ShamirShare, SharedSecret};

// Re-export helper functions from helpers module
pub use crate::helpers::{deserialize_secret_key, serialize_secret_key};

/// A BFV-encrypted Shamir share for secure transmission.
///
/// Each share is encrypted as multiple BFV ciphertexts (one per modulus level).
/// The recipient can only decrypt using their corresponding BFV secret key.
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct BfvEncryptedShare {
    /// BFV ciphertexts, one per modulus level
    #[derivative(Debug(format_with = "debug_vec_arcbytes"))]
    ciphertexts: Vec<ArcBytes>,
}

/// Debug helper for Vec<ArcBytes>
fn debug_vec_arcbytes(v: &Vec<ArcBytes>, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(
        f,
        "[{} ciphertexts, total {} bytes]",
        v.len(),
        v.iter().map(|c| c.len()).sum::<usize>()
    )
}

impl BfvEncryptedShare {
    /// Encrypt a Shamir share for a specific recipient.
    ///
    /// # Arguments
    /// * `share` - The Shamir share to encrypt (contains data for all moduli)
    /// * `recipient_pk` - The recipient's BFV public key
    /// * `params` - BFV parameters for share encryption
    /// * `rng` - Random number generator
    ///
    /// # Returns
    /// An encrypted share that can only be decrypted by the recipient
    pub fn encrypt<R: RngCore + CryptoRng>(
        share: &ShamirShare,
        recipient_pk: &PublicKey,
        params: &Arc<BfvParameters>,
        rng: &mut R,
    ) -> Result<Self> {
        let data: &Array2<u64> = share.deref(); // Array2<u64> with rows = moduli, cols = coefficients
        let num_moduli = data.nrows();

        let mut ciphertexts = Vec::with_capacity(num_moduli);

        for m in 0..num_moduli {
            let row = data.row(m);
            let share_vec: Vec<u64> = row.to_vec();

            let pt = Plaintext::try_encode(&share_vec, Encoding::poly(), params)
                .context("Failed to encode share as plaintext")?;

            let ct = recipient_pk
                .try_encrypt(&pt, rng)
                .context("Failed to encrypt share")?;

            ciphertexts.push(ArcBytes::from_bytes(&ct.to_bytes()));
        }

        Ok(Self { ciphertexts })
    }

    /// Decrypt an encrypted share using the recipient's secret key.
    ///
    /// # Arguments
    /// * `sk` - The recipient's BFV secret key
    /// * `params` - BFV parameters for share encryption
    /// * `degree` - Polynomial degree (for reconstructing the share matrix)
    ///
    /// # Returns
    /// The decrypted Shamir share
    pub fn decrypt(
        self,
        sk: &SecretKey,
        params: &Arc<BfvParameters>,
        degree: usize,
    ) -> Result<ShamirShare> {
        let num_moduli = self.ciphertexts.len();
        let mut data = Array2::zeros((num_moduli, degree));

        for (m, ct_bytes) in self.ciphertexts.into_iter().enumerate() {
            let ct = Ciphertext::from_bytes(&ct_bytes, params)
                .context("Failed to deserialize ciphertext")?;

            let pt = sk
                .try_decrypt(&ct)
                .context("Failed to decrypt ciphertext")?;

            let decrypted: Vec<u64> = Vec::<u64>::try_decode(&pt, Encoding::poly())
                .context("Failed to decode plaintext")?;

            for (i, val) in decrypted.into_iter().take(degree).enumerate() {
                data[[m, i]] = val;
            }
        }

        Ok(ShamirShare::new(data))
    }
}

impl Default for BfvEncryptedShare {
    fn default() -> Self {
        Self {
            ciphertexts: Vec::new(),
        }
    }
}

/// A collection of BFV-encrypted shares for all recipients.
///
/// When a party generates Shamir shares, they encrypt each recipient's share
/// with that recipient's public key. This struct holds all encrypted shares
/// from a single sender.
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct BfvEncryptedShares {
    /// Encrypted shares indexed by recipient party_id (0-based)
    shares: Vec<BfvEncryptedShare>,
}

impl BfvEncryptedShares {
    /// Encrypt shares for all recipients.
    ///
    /// # Arguments
    /// * `secret` - The SharedSecret containing shares for all parties
    /// * `recipient_pks` - Public keys for all recipients, indexed by party_id
    /// * `params` - BFV parameters for share encryption
    /// * `rng` - Random number generator
    pub fn encrypt_all<R: RngCore + CryptoRng>(
        secret: &SharedSecret,
        recipient_pks: &[PublicKey],
        params: &Arc<BfvParameters>,
        rng: &mut R,
    ) -> Result<Self> {
        let num_parties = recipient_pks.len();
        let mut shares = Vec::with_capacity(num_parties);

        for party_id in 0..num_parties {
            let share = secret
                .extract_party_share(party_id)
                .context(format!("Failed to extract share for party {}", party_id))?;

            let encrypted =
                BfvEncryptedShare::encrypt(&share, &recipient_pks[party_id], params, rng)?;

            shares.push(encrypted);
        }

        Ok(Self { shares })
    }

    /// Get the encrypted share for a specific recipient.
    pub fn get_share(&self, party_id: usize) -> Option<&BfvEncryptedShare> {
        self.shares.get(party_id)
    }

    /// Clone the encrypted share for a specific recipient.
    pub fn clone_share(&self, party_id: usize) -> Option<BfvEncryptedShare> {
        self.shares.get(party_id).cloned()
    }

    /// Extract only the share for a specific party (for bandwidth optimization)
    pub fn extract_for_party(&self, party_id: usize) -> Option<Self> {
        self.shares.get(party_id).map(|share| Self {
            shares: vec![share.clone()],
        })
    }

    /// Number of encrypted shares
    pub fn len(&self) -> usize {
        self.shares.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.shares.is_empty()
    }
}

impl Default for BfvEncryptedShares {
    fn default() -> Self {
        Self { shares: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_fhe_params::{BfvParamSet, BfvPreset};
    use rand::rngs::OsRng;

    #[test]
    fn test_encrypt_decrypt_share() {
        let params = BfvParamSet::from(BfvPreset::InsecureDkg512).build_arc();
        let mut rng = OsRng;

        // Generate key pair
        let sk = SecretKey::random(&params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        // Create a test share (1 modulus row, 512 coefficients)
        let degree = params.degree();
        let test_data: Vec<u64> = (0..degree as u64).collect();
        let mut data = Array2::zeros((1, degree));
        for (i, val) in test_data.iter().enumerate() {
            data[[0, i]] = *val;
        }
        let share = ShamirShare::new(data.clone());

        // Encrypt
        let encrypted = BfvEncryptedShare::encrypt(&share, &pk, &params, &mut rng)
            .expect("Encryption should succeed");

        // Decrypt
        let decrypted = encrypted
            .decrypt(&sk, &params, degree)
            .expect("Decryption should succeed");

        // Verify
        assert_eq!(share.deref(), decrypted.deref());
    }

    #[test]
    fn test_secret_key_serialization() {
        let params = BfvParamSet::from(BfvPreset::InsecureDkg512).build_arc();
        let mut rng = OsRng;

        // Generate a secret key
        let sk = SecretKey::random(&params, &mut rng);

        // Serialize
        let bytes = serialize_secret_key(&sk).expect("Serialization should succeed");

        // Deserialize
        let sk_restored =
            deserialize_secret_key(&bytes, &params).expect("Deserialization should succeed");

        // Verify coefficients match
        assert_eq!(sk.coeffs, sk_restored.coeffs);
    }
}
