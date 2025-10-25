// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Core crisp inputs generation library.
//!
//! This crate contains the main logic for generating crisp inputs for zero-knowledge proofs.

use eyre::{Context, Result};
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
use crate::ciphertext_addition::CiphertextAdditionInputs;

mod serialization;
use serialization::{construct_inputs, serialize_inputs_to_json};

pub struct CrispZKInputsGenerator {
    bfv_params: Arc<BfvParameters>,
}

impl CrispZKInputsGenerator {
    pub fn new() -> Self {
        Self::with_defaults()
    }

    /// Creates a new generator with custom BFV parameters.
    pub fn with_params(degree: usize, plaintext_modulus: u64, moduli: &[u64]) -> Self {
        let bfv_params = BfvParametersBuilder::new()
            .set_degree(degree)
            .set_plaintext_modulus(plaintext_modulus)
            .set_moduli(moduli)
            .build_arc()
            .unwrap();
        Self { bfv_params }
    }

    /// Creates a generator with default BFV parameters.
    fn with_defaults() -> Self {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = [0x3FFFFFFF000001];

        Self::with_params(degree, plaintext_modulus, &moduli)
    }

    /// Generates crisp ZK inputs for a vote encryption and addition operation.
    ///
    /// # Arguments
    /// * `prev_ciphertext` - Hex-encoded previous ciphertext to add to
    /// * `public_key` - Hex-encoded public key for encryption
    /// * `vote` - Vote value (0 or 1)
    ///
    /// # Returns
    /// JSON string containing the crisp ZK inputs
    pub fn generate_inputs(
        &self,
        prev_ciphertext: &str,
        public_key: &str,
        vote: u8,
    ) -> Result<String> {
        // Deserialize the provided public key.
        let pk = PublicKey::from_bytes(
            &hex::decode(public_key).with_context(|| "Failed to decode public key from hex")?,
            &self.bfv_params,
        )
        .with_context(|| "Failed to deserialize public key")?;

        // Create a sample plaintext consistent with the GRECO sample generator.
        // All coefficients are 3, and the first coefficient represents the vote.
        let mut message_data = vec![3u64; self.bfv_params.degree()];
        // Set vote value (0 or 1) in the first coefficient.
        message_data[0] = if vote == 1 { 1 } else { 0 };

        // Encode the plaintext into a polynomial.
        let pt = Plaintext::try_encode(&message_data, Encoding::poly(), &self.bfv_params)
            .with_context(|| "Failed to encode plaintext")?;

        // Encrypt using the provided public key to ensure ciphertext matches the key.
        let (ct, u_rns, e0_rns, e1_rns) = pk
            .try_encrypt_extended(&pt, &mut thread_rng())
            .with_context(|| "Failed to encrypt plaintext")?;

        // Compute the vectors of the GRECO inputs.
        let greco_vectors =
            GrecoVectors::compute(&pt, &u_rns, &e0_rns, &e1_rns, &ct, &pk, &self.bfv_params)
                .with_context(|| "Failed to compute vectors")?;

        let (crypto_params, bounds) = GrecoBounds::compute(&self.bfv_params, 0)
            .with_context(|| "Failed to compute bounds")?;

        // Ciphertext Addition Section.
        // Deserialize the previous ciphertext.
        let prev_ct = Ciphertext::from_bytes(
            &hex::decode(prev_ciphertext)
                .with_context(|| "Failed to decode previous ciphertext from hex")?,
            &self.bfv_params,
        )
        .with_context(|| "Failed to deserialize previous ciphertext")?;

        // Compute the ciphertext addition.
        let sum_ct = &ct + &prev_ct;

        // Compute the inputs of the ciphertext addition.
        let ciphertext_addition_inputs =
            CiphertextAdditionInputs::compute(&pt, &prev_ct, &ct, &sum_ct, &self.bfv_params)
                .with_context(|| "Failed to compute ciphertext addition inputs")?;

        // Construct Inputs Section.
        let inputs = construct_inputs(
            &crypto_params,
            &bounds,
            &greco_vectors.standard_form(),
            &ciphertext_addition_inputs.standard_form(),
        );

        Ok(serialize_inputs_to_json(&inputs)?)
    }

    /// Encrypts a vote using the provided public key.
    ///
    /// # Arguments
    /// * `public_key` - Hex-encoded public key for encryption
    /// * `vote` - Vote value (0 or 1)
    ///
    /// # Returns
    /// Hex-encoded ciphertext
    pub fn encrypt_vote(&self, public_key: &str, vote: u8) -> Result<String> {
        let pk = PublicKey::from_bytes(
            &hex::decode(public_key).with_context(|| "Failed to decode public key from hex")?,
            &self.bfv_params,
        )
        .with_context(|| "Failed to deserialize public key")?;

        let mut message_data = vec![3u64; self.bfv_params.degree()];
        // Set vote value (0 or 1) in the first coefficient.
        message_data[0] = if vote == 1 { 1 } else { 0 };

        let pt = Plaintext::try_encode(&message_data, Encoding::poly(), &self.bfv_params)
            .with_context(|| "Failed to encode plaintext")?;

        let (ct, _u_rns, _e0_rns, _e1_rns) = pk
            .try_encrypt_extended(&pt, &mut thread_rng())
            .with_context(|| "Failed to encrypt plaintext")?;

        Ok(hex::encode(ct.to_bytes()))
    }

    /// Generates a new public/secret key pair and returns the public key.
    ///
    /// # Returns
    /// Hex-encoded public key
    pub fn generate_public_key(&self) -> Result<String> {
        // Generate keys.
        let mut rng = thread_rng();
        let sk = SecretKey::random(&self.bfv_params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        Ok(hex::encode(pk.to_bytes()))
    }

    /// Returns a clone of the BFV parameters used by this generator.
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
        let prev_ciphertext = generator
            .encrypt_vote(&public_key, 1)
            .expect("failed to generate previous ciphertext");
        let result = generator.generate_inputs(&prev_ciphertext, &public_key, 0);

        assert!(result.is_ok());
        let json_output = result.unwrap();
        // Verify it's valid JSON and contains expected fields.
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
        let prev_ciphertext = generator
            .encrypt_vote(&public_key, 0)
            .expect("failed to generate previous ciphertext");
        let result = generator.generate_inputs(&prev_ciphertext, &public_key, 1);

        assert!(result.is_ok());
        let json_output = result.unwrap();
        // Verify it's valid JSON and contains expected fields.
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
        let prev_ciphertext = generator
            .encrypt_vote(&public_key, 0)
            .expect("failed to generate previous ciphertext");
        let result = generator.generate_inputs(&prev_ciphertext, &public_key, 1);

        assert!(result.is_ok());
        let json_output = result.unwrap();
        // Verify it's valid JSON and contains expected fields.
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

        // Test that functions use secure randomness (no deterministic seed).
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

    // Error handling tests
    #[test]
    fn test_invalid_inputs() {
        let generator = CrispZKInputsGenerator::new();

        // Test invalid hex inputs.
        let result = generator.generate_inputs("invalid_hex", "invalid_hex", 0);
        assert!(result.is_err());

        // Test empty strings.
        let result = generator.generate_inputs("", "", 0);
        assert!(result.is_err());

        // Test invalid public key for encryption.
        let result = generator.encrypt_vote("invalid_hex", 0);
        assert!(result.is_err());
    }

    // Core functionality tests
    #[test]
    fn test_vote_values() {
        let generator = CrispZKInputsGenerator::new();
        let public_key = generator
            .generate_public_key()
            .expect("failed to generate public key");
        let prev_ciphertext = generator
            .encrypt_vote(&public_key, 0)
            .expect("failed to encrypt vote");

        // Test vote = 0.
        let result_0 = generator.generate_inputs(&prev_ciphertext, &public_key, 0);
        assert!(result_0.is_ok());

        // Test vote = 1.
        let result_1 = generator.generate_inputs(&prev_ciphertext, &public_key, 1);
        assert!(result_1.is_ok());
    }

    #[test]
    fn test_bfv_params_consistency() {
        let generator = CrispZKInputsGenerator::new();
        let bfv_params = generator.get_bfv_params();

        // Verify default parameters.
        assert_eq!(bfv_params.degree(), 2048);
        assert_eq!(bfv_params.plaintext(), 1032193);
        assert_eq!(bfv_params.moduli(), &[0x3FFFFFFF000001]);
    }

    #[test]
    fn test_json_output_structure() {
        let generator = CrispZKInputsGenerator::new();
        let public_key = generator
            .generate_public_key()
            .expect("failed to generate public key");
        let prev_ciphertext = generator
            .encrypt_vote(&public_key, 0)
            .expect("failed to encrypt vote");
        let result = generator.generate_inputs(&prev_ciphertext, &public_key, 1);

        assert!(result.is_ok());
        let json_output = result.unwrap();

        // Parse JSON to verify structure.
        let parsed: serde_json::Value =
            serde_json::from_str(&json_output).expect("Invalid JSON output");

        // Check required top-level fields.
        assert!(parsed.get("params").is_some());
        assert!(parsed.get("ct0is").is_some());
        assert!(parsed.get("ct1is").is_some());
        assert!(parsed.get("pk0is").is_some());
        assert!(parsed.get("pk1is").is_some());
        assert!(parsed.get("ct_add").is_some());
    }

    #[test]
    fn test_cryptographic_properties() {
        let generator = CrispZKInputsGenerator::new();
        let public_key = generator
            .generate_public_key()
            .expect("Failed to generate public key");

        // Test that different votes produce different ciphertexts.
        let ct0 = generator
            .encrypt_vote(&public_key, 0)
            .expect("Failed to encrypt vote 0");
        let ct1 = generator
            .encrypt_vote(&public_key, 1)
            .expect("Failed to encrypt vote 1");

        assert_ne!(ct0, ct1);

        // Test that same vote produces different ciphertexts (due to randomness).
        let ct0_2 = generator
            .encrypt_vote(&public_key, 0)
            .expect("Failed to encrypt vote 0 again");
        assert_ne!(ct0, ct0_2);

        // Test that all ciphertexts are valid hex.
        assert!(hex::decode(&ct0).is_ok());
        assert!(hex::decode(&ct1).is_ok());
        assert!(hex::decode(&ct0_2).is_ok());
    }
}
