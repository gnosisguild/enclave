// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Core crisp inputs generation library
//!
//! This crate contains the main logic for generating crisp inputs.

use fhe::bfv::Ciphertext;
use fhe::bfv::PublicKey;
use fhe::bfv::SecretKey;
use fhe::bfv::{BfvParameters, BfvParametersBuilder};
use fhe::bfv::{Encoding, Plaintext};
use fhe_traits::{DeserializeParametrized, FheEncoder, Serialize};
use greco::bounds::GrecoBounds;
use greco::vectors::GrecoVectors;
use rand::thread_rng;
use std::sync::Arc;

mod ciphertext_addition;
use crate::ciphertext_addition::CiphertextAdditionParams;

mod serialization;
use serialization::{construct_inputs, serialize_inputs_to_json};

pub struct CrispZKInputsGenerator {
    bfv_params: Arc<BfvParameters>,
}

impl CrispZKInputsGenerator {
    pub fn new() -> Self {
        Self::with_defaults()
    }

    pub fn with_params(degree: usize, plaintext_modulus: u64, moduli: &[u64]) -> Self {
        let bfv_params = BfvParametersBuilder::new()
            .set_degree(degree)
            .set_plaintext_modulus(plaintext_modulus)
            .set_moduli(moduli)
            .build_arc()
            .unwrap();
        Self { bfv_params }
    }

    fn with_defaults() -> Self {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = [0x3FFFFFFF000001];

        Self::with_params(degree, plaintext_modulus, &moduli)
    }

    pub fn generate_inputs(
        &self,
        old_ciphertext: &str,
        public_key: &str,
        vote: u8,
    ) -> Result<String, String> {
        // Deserialize the provided public key
        let pk = PublicKey::from_bytes(
            &hex::decode(public_key).map_err(|e| format!("Failed to decode public key: {}", e))?,
            &self.bfv_params,
        )
        .map_err(|e| format!("Failed to deserialize public key: {}", e))?;

        // Create a sample plaintext consistent with the GRECO sample generator
        // All coefficients are 3, and the first coefficient represents the vote.
        let mut message_data = vec![3u64; self.bfv_params.degree()];
        // Set vote value (0 or 1) in the first coefficient
        message_data[0] = if vote == 1 { 1 } else { 0 };

        // Encode the plaintext into a polynomial.
        let pt = Plaintext::try_encode(&message_data, Encoding::poly(), &self.bfv_params)
            .map_err(|e| format!("Failed to encode plaintext: {}", e))?;

        // Encrypt using the provided public key to ensure ciphertext matches the key.
        let (ct, u_rns, e0_rns, e1_rns) = pk
            .try_encrypt_extended(&pt, &mut thread_rng())
            .map_err(|e| format!("Failed to encrypt plaintext: {}", e))?;

        // Compute the vectors of the Greco inputs.
        let greco_vectors =
            GrecoVectors::compute(&pt, &u_rns, &e0_rns, &e1_rns, &ct, &pk, &self.bfv_params)
                .map_err(|e| format!("Failed to compute vectors: {}", e))?;

        let (crypto_params, bounds) = GrecoBounds::compute(&self.bfv_params, 0)
            .map_err(|e| format!("Failed to compute bounds: {}", e))?;

        // #########################################  Ciphertext Addition ################################################

        // Deserialize the old ciphertext.
        let old_ct = Ciphertext::from_bytes(
            &hex::decode(old_ciphertext)
                .map_err(|e| format!("Failed to decode old ciphertext: {}", e))?,
            &self.bfv_params,
        )
        .map_err(|e| format!("Failed to deserialize old ciphertext: {}", e))?;

        // Compute the cyphertext addition.
        let sum_ct = &ct + &old_ct;

        // Compute the vectors of the ciphertext addition inputs.
        let ciphertext_addition_vectors =
            CiphertextAdditionParams::compute(&pt, &old_ct, &ct, &sum_ct, &self.bfv_params)
                .map_err(|e| format!("Failed to compute ciphertext addition vectors: {}", e))?;

        // #########################################  Construct Inputs ################################################

        let inputs = construct_inputs(
            &crypto_params,
            &bounds,
            &greco_vectors.standard_form(),
            &ciphertext_addition_vectors.standard_form(),
        );

        Ok(serialize_inputs_to_json(&inputs)?)
    }

    pub fn encrypt_vote(&self, public_key: &str, vote: u8) -> Result<String, String> {
        let pk = PublicKey::from_bytes(
            &hex::decode(public_key).map_err(|e| format!("Failed to decode public key: {}", e))?,
            &self.bfv_params,
        )
        .map_err(|e| format!("Failed to deserialize public key: {}", e))?;

        let mut message_data = vec![3u64; self.bfv_params.degree()];
        // Set vote value (0 or 1) in the first coefficient
        message_data[0] = if vote == 1 { 1 } else { 0 };

        let pt = Plaintext::try_encode(&message_data, Encoding::poly(), &self.bfv_params)
            .map_err(|e| format!("Failed to encode plaintext: {}", e))?;

        let (ct, _u_rns, _e0_rns, _e1_rns) = pk
            .try_encrypt_extended(&pt, &mut thread_rng())
            .map_err(|e| format!("Failed to encrypt plaintext: {}", e))?;

        Ok(hex::encode(ct.to_bytes()))
    }

    pub fn generate_public_key(&self) -> Result<String, String> {
        // Generate keys
        let mut rng = thread_rng();
        let sk = SecretKey::random(&self.bfv_params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        Ok(hex::encode(pk.to_bytes()))
    }

    pub fn get_bfv_params(&self) -> Arc<BfvParameters> {
        self.bfv_params.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inputs_generation_with_defaults() {
        let generator = CrispZKInputsGenerator::new();
        let public_key = generator
            .generate_public_key()
            .expect("failed to generate public key");
        let old_ciphertext = generator
            .encrypt_vote(&public_key, 1)
            .expect("failed to generate old ciphertext");
        let result = generator.generate_inputs(&old_ciphertext, &public_key, 0);

        assert!(result.is_ok());
        let json_output = result.unwrap();
        // Verify it's valid JSON and contains expected fields
        assert!(json_output.contains("params"));
        assert!(json_output.contains("pk0is"));
        assert!(json_output.contains("crypto"));
    }

    #[test]
    fn test_inputs_generation_with_custom_params() {
        let generator = CrispZKInputsGenerator::with_params(2048, 1032193, &[0x3FFFFFFF000001]);
        let public_key = generator
            .generate_public_key()
            .expect("failed to generate public key");
        let old_ciphertext = generator
            .encrypt_vote(&public_key, 0)
            .expect("failed to generate old ciphertext");
        let result = generator.generate_inputs(&old_ciphertext, &public_key, 1);

        assert!(result.is_ok());
        let json_output = result.unwrap();
        // Verify it's valid JSON and contains expected fields
        assert!(json_output.contains("params"));
        assert!(json_output.contains("pk0is"));
        assert!(json_output.contains("crypto"));
    }

    #[test]
    fn test_inputs_generation_with_vote_0() {
        let generator = CrispZKInputsGenerator::new();
        let public_key = generator
            .generate_public_key()
            .expect("failed to generate public key");
        let old_ciphertext = generator
            .encrypt_vote(&public_key, 0)
            .expect("failed to generate old ciphertext");
        let result = generator.generate_inputs(&old_ciphertext, &public_key, 1);

        assert!(result.is_ok());
        let json_output = result.unwrap();
        // Verify it's valid JSON and contains expected fields
        assert!(json_output.contains("params"));
        assert!(json_output.contains("pk0is"));
        assert!(json_output.contains("crypto"));
    }

    #[test]
    fn test_get_bfv_params() {
        let generator = CrispZKInputsGenerator::new();
        let bfv_params = generator.get_bfv_params();

        assert!(bfv_params.degree() == 2048);
        assert!(bfv_params.plaintext() == 1032193 as u64);
        assert!(bfv_params.moduli() == &[0x3FFFFFFF000001]);
    }

    #[test]
    fn test_secure_rng_usage() {
        let generator = CrispZKInputsGenerator::new();

        // Test that functions use secure randomness (no deterministic seed)
        let public_key = generator
            .generate_public_key()
            .expect("failed to generate public key");
        assert!(!public_key.is_empty());

        let ciphertext = generator
            .encrypt_vote(&public_key, 1)
            .expect("failed to encrypt vote");
        assert!(!ciphertext.is_empty());

        let result = generator.generate_inputs(&ciphertext, &public_key, 0);
        assert!(result.is_ok());
        let json_output = result.unwrap();
        assert!(json_output.contains("params"));
        assert!(json_output.contains("pk0is"));
        assert!(json_output.contains("crypto"));
    }
}
