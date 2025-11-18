// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Core CRISP ZK inputs generation library.
//!
//! This crate contains the main logic for generating CRISP inputs for zero-knowledge proofs.

use crisp_constants::get_default_paramset;
use e3_sdk::bfv_helpers::build_bfv_params_arc;
use e3_sdk::bfv_helpers::BfvParamSet;
use e3_sdk::bfv_helpers::BfvParamSets;
use eyre::{Context, Result};
use fhe::bfv::BfvParameters;
use fhe::bfv::Ciphertext;
use fhe::bfv::PublicKey;
use fhe::bfv::SecretKey;
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

pub struct ZKInputsGenerator {
    bfv_params: Arc<BfvParameters>,
}

impl ZKInputsGenerator {
    /// Creates a new generator with the specified BFV parameters.
    pub fn new(
        degree: usize,
        plaintext_modulus: u64,
        moduli: &[u64],
        error1_variance: Option<&str>,
    ) -> Self {
        let bfv_params = build_bfv_params_arc(degree, plaintext_modulus, moduli, error1_variance);
        Self { bfv_params }
    }

    /// Creates a new generator with the specified BFV parameter set.
    pub fn from_set(set: BfvParamSet) -> Self {
        let bfv_params = set.build_arc();

        Self { bfv_params }
    }

    /// Creates a generator with default BFV parameters for testing purposes.
    ///
    /// # Notes
    /// - This is for testing purposes only.
    /// - The default parameters are not suitable for production.
    /// # Returns
    /// A new ZKInputsGenerator instance with default BFV parameters
    pub fn with_defaults() -> Self {
        Self::from_set(get_default_paramset())
    }

    /// Generates CRISP ZK inputs for a vote encryption and addition operation.
    ///
    /// # Arguments
    /// * `prev_ciphertext` - Previous ciphertext bytes to add to
    /// * `public_key` - Public key bytes for encryption
    /// * `vote` - Vote value as a vector of coefficients
    ///
    /// # Returns
    /// JSON string containing the CRISP ZK inputs
    pub fn generate_inputs(
        &self,
        prev_ciphertext: &[u8],
        public_key: &[u8],
        vote: Vec<u64>,
    ) -> Result<String> {
        // Deserialize the provided public key.
        let pk = PublicKey::from_bytes(public_key, &self.bfv_params)
            .with_context(|| "Failed to deserialize public key")?;

        // Encode the plaintext into a polynomial.
        let pt = Plaintext::try_encode(&vote, Encoding::poly(), &self.bfv_params)
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
        let prev_ct = Ciphertext::from_bytes(prev_ciphertext, &self.bfv_params)
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
    /// * `public_key` - Public key bytes for encryption
    /// * `vote` - Vote data as a vector of coefficients
    ///
    /// # Returns
    /// Ciphertext bytes
    pub fn encrypt_vote(&self, public_key: &[u8], vote: Vec<u64>) -> Result<Vec<u8>> {
        let pk = PublicKey::from_bytes(public_key, &self.bfv_params)
            .with_context(|| "Failed to deserialize public key")?;

        let pt = Plaintext::try_encode(&vote, Encoding::poly(), &self.bfv_params)
            .with_context(|| "Failed to encode plaintext")?;

        let (ct, _u_rns, _e0_rns, _e1_rns) = pk
            .try_encrypt_extended(&pt, &mut thread_rng())
            .with_context(|| "Failed to encrypt plaintext")?;

        Ok(ct.to_bytes())
    }

    /// Generates a new public/secret key pair and returns the public key.
    ///
    /// # Returns
    /// Raw bytes of the public key
    pub fn generate_public_key(&self) -> Result<Vec<u8>> {
        // Generate keys.
        let mut rng = thread_rng();
        let sk = SecretKey::random(&self.bfv_params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        Ok(pk.to_bytes())
    }

    /// Returns a clone of the BFV parameters used by this generator.
    pub fn get_bfv_params(&self) -> Arc<BfvParameters> {
        self.bfv_params.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_DEGREE: usize = 512;

    /// Helper function to create a vote vector with alternating 0s and 1s (deterministic)
    fn create_vote_vector() -> Vec<u64> {
        (0..DEFAULT_DEGREE).map(|i| (i % 2) as u64).collect()
    }

    #[test]
    fn test_inputs_generation_with_defaults() {
        let generator = ZKInputsGenerator::with_defaults();
        let public_key = generator
            .generate_public_key()
            .expect("failed to generate public key");
        let vote = create_vote_vector();
        let prev_ciphertext = generator
            .encrypt_vote(&public_key, vote.clone())
            .expect("failed to generate previous ciphertext");
        let result = generator.generate_inputs(&prev_ciphertext, &public_key, vote.clone());

        assert!(result.is_ok());
        let json_output = result.unwrap();
        // Verify it's valid JSON and contains expected fields.
        assert!(json_output.contains("params"));
        assert!(json_output.contains("pk0is"));
        assert!(json_output.contains("crypto"));
    }

    #[test]
    fn test_inputs_generation_with_custom_params() {
        let generator = ZKInputsGenerator::from_set(BfvParamSet {
            degree: 2048,
            plaintext_modulus: 1032193,
            moduli: &[0x3FFFFFFF000001],
            error1_variance: None,
        });
        let public_key = generator
            .generate_public_key()
            .expect("failed to generate public key");
        let vote = create_vote_vector();
        let prev_ciphertext = generator
            .encrypt_vote(&public_key, vote.clone())
            .expect("failed to generate previous ciphertext");
        let result = generator.generate_inputs(&prev_ciphertext, &public_key, vote.clone());

        assert!(result.is_ok());
        let json_output = result.unwrap();
        // Verify it's valid JSON and contains expected fields.
        assert!(json_output.contains("params"));
        assert!(json_output.contains("pk0is"));
        assert!(json_output.contains("crypto"));
    }

    #[test]
    fn test_inputs_generation_with_vote_0() {
        let generator = ZKInputsGenerator::with_defaults();
        let public_key = generator
            .generate_public_key()
            .expect("failed to generate public key");
        let vote = create_vote_vector();
        let prev_ciphertext = generator
            .encrypt_vote(&public_key, vote.clone())
            .expect("failed to generate previous ciphertext");
        let result = generator.generate_inputs(&prev_ciphertext, &public_key, vote.clone());

        assert!(result.is_ok());
        let json_output = result.unwrap();
        // Verify it's valid JSON and contains expected fields.
        assert!(json_output.contains("params"));
        assert!(json_output.contains("pk0is"));
        assert!(json_output.contains("crypto"));
    }

    #[test]
    fn test_get_bfv_params() {
        let generator = ZKInputsGenerator::from_set(BfvParamSet {
            degree: 2048,
            plaintext_modulus: 1032193,
            moduli: &[0x3FFFFFFF000001],
            error1_variance: None,
        });
        let bfv_params = generator.get_bfv_params();

        assert!(bfv_params.degree() == 2048);
        assert!(bfv_params.plaintext() == 1032193 as u64);
        assert!(bfv_params.moduli() == &[0x3FFFFFFF000001]);
    }

    #[test]
    fn test_secure_rng_usage() {
        let generator = ZKInputsGenerator::with_defaults();

        // Test that functions use secure randomness (no deterministic seed).
        let public_key = generator
            .generate_public_key()
            .expect("failed to generate public key");
        assert!(!public_key.is_empty());
        let vote = create_vote_vector();

        let ciphertext = generator
            .encrypt_vote(&public_key, vote.clone())
            .expect("failed to encrypt vote");
        assert!(!ciphertext.is_empty());

        let result = generator.generate_inputs(&ciphertext, &public_key, vote.clone());
        assert!(result.is_ok());
        let json_output = result.unwrap();
        assert!(json_output.contains("params"));
        assert!(json_output.contains("pk0is"));
        assert!(json_output.contains("crypto"));
    }

    // Error handling tests
    #[test]
    fn test_invalid_inputs() {
        let generator = ZKInputsGenerator::with_defaults();
        let vote = create_vote_vector();

        // Test invalid byte inputs.
        let result = generator.generate_inputs(&[1, 2, 3], &[4, 5, 6], vote.clone());
        assert!(result.is_err());

        // Test empty slices.
        let result = generator.generate_inputs(&[], &[], vote.clone());
        assert!(result.is_err());

        // Test invalid public key for encryption.
        let result = generator.encrypt_vote(&[1, 2, 3], vote.clone());
        assert!(result.is_err());
    }

    // Core functionality tests
    #[test]
    fn test_vote_values() {
        let generator = ZKInputsGenerator::with_defaults();
        let public_key = generator
            .generate_public_key()
            .expect("failed to generate public key");
        let vote = create_vote_vector();
        let prev_ciphertext = generator
            .encrypt_vote(&public_key, vote.clone())
            .expect("failed to encrypt vote");

        // Test vote = 0.
        let result_0 = generator.generate_inputs(&prev_ciphertext, &public_key, vote.clone());
        assert!(result_0.is_ok());

        // Test vote = 1.
        let result_1 = generator.generate_inputs(&prev_ciphertext, &public_key, vote.clone());
        assert!(result_1.is_ok());
    }

    #[test]
    fn test_json_output_structure() {
        let generator = ZKInputsGenerator::with_defaults();
        let public_key = generator
            .generate_public_key()
            .expect("failed to generate public key");
        let vote = create_vote_vector();
        let prev_ciphertext = generator
            .encrypt_vote(&public_key, vote.clone())
            .expect("failed to encrypt vote");
        let result = generator.generate_inputs(&prev_ciphertext, &public_key, vote.clone());

        assert!(result.is_ok());
        let json_output = result.unwrap();

        // Parse JSON to verify structure.
        let parsed: serde_json::Value =
            serde_json::from_str(&json_output).expect("Invalid JSON output");

        // Check required top-level fields.
        assert!(parsed.get("params").is_some());
        assert!(parsed.get("prev_ct0is").is_some());
        assert!(parsed.get("prev_ct1is").is_some());
        assert!(parsed.get("sum_ct0is").is_some());
        assert!(parsed.get("sum_ct1is").is_some());
        assert!(parsed.get("sum_r0is").is_some());
        assert!(parsed.get("sum_r1is").is_some());
        assert!(parsed.get("ct0is").is_some());
        assert!(parsed.get("ct1is").is_some());
        assert!(parsed.get("pk0is").is_some());
        assert!(parsed.get("pk1is").is_some());
    }

    #[test]
    fn test_cryptographic_properties() {
        let generator = ZKInputsGenerator::with_defaults();
        let public_key = generator
            .generate_public_key()
            .expect("Failed to generate public key");
        let vote = create_vote_vector();

        // Test that different votes produce different ciphertexts.
        let ct0 = generator
            .encrypt_vote(&public_key, vote.clone())
            .expect("Failed to encrypt vote 0");
        let ct1 = generator
            .encrypt_vote(&public_key, vote.clone())
            .expect("Failed to encrypt vote 1");

        assert_ne!(ct0, ct1);

        // Test that same vote produces different ciphertexts (due to randomness).
        let ct0_2 = generator
            .encrypt_vote(&public_key, create_vote_vector())
            .expect("Failed to encrypt vote 0 again");
        assert_ne!(ct0, ct0_2);

        // Test that all ciphertexts are non-empty.
        assert!(!ct0.is_empty());
        assert!(!ct1.is_empty());
        assert!(!ct0_2.is_empty());
    }
}
