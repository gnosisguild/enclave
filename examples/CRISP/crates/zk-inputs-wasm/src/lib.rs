// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! JavaScript library with WASM bindings for CRISP ZK inputs generation.
//!
//! This crate provides JavaScript bindings for the CRISP ZK inputs generator using WASM.

use e3_polynomial::CrtPolynomial;
use js_sys;
use num_bigint::BigInt;
use wasm_bindgen::prelude::*;
use zk_inputs::ZKInputsGenerator as CoreZKInputsGenerator;

/// JavaScript-compatible CRISP ZK inputs generator.
#[wasm_bindgen]
pub struct ZKInputsGenerator {
    generator: CoreZKInputsGenerator,
}

#[wasm_bindgen]
impl ZKInputsGenerator {
    /// Create a new JavaScript CRISP ZK inputs generator with the specified BFV parameters.
    ///
    /// # Arguments
    /// - `degree`: Polynomial degree
    /// - `plaintext_modulus`: Plaintext modulus (will be converted to u64)
    /// - `moduli`: Array of moduli (will be converted to Vec<u64>)
    #[wasm_bindgen(constructor)]
    pub fn new(
        degree: usize,
        plaintext_modulus: i64,
        moduli: Vec<i64>,
    ) -> Result<ZKInputsGenerator, JsValue> {
        let plaintext_modulus_u64 = plaintext_modulus as u64;
        let moduli_vec: Vec<u64> = moduli.into_iter().map(|m| m as u64).collect();
        // Should we pass an error1_variance here?
        let generator =
            CoreZKInputsGenerator::new(degree, plaintext_modulus_u64, &moduli_vec, None);
        Ok(ZKInputsGenerator { generator })
    }

    /// Create a new JavaScript CRISP ZK inputs generator with default BFV parameters.
    #[wasm_bindgen(js_name = "withDefaults")]
    pub fn with_defaults() -> Result<ZKInputsGenerator, JsValue> {
        let generator = CoreZKInputsGenerator::with_defaults();
        Ok(ZKInputsGenerator { generator })
    }

    /// Generate CRISP ZK inputs from JavaScript.
    #[wasm_bindgen(js_name = "generateInputs")]
    pub fn generate_inputs(
        &self,
        prev_ciphertext: &[u8],
        public_key: &[u8],
        vote: Vec<i64>,
    ) -> Result<JsValue, JsValue> {
        let vote_vec: Vec<u64> = vote.into_iter().map(|v| v as u64).collect();

        match self
            .generator
            .generate_inputs(prev_ciphertext, public_key, vote_vec)
        {
            Ok((ciphertext_bytes, inputs_json)) => {
                // Parse the JSON string and return as an object with both encryptedVote and inputs.
                let result = js_sys::Object::new();

                // Set encryptedVote as Uint8Array
                let ciphertext_array = js_sys::Uint8Array::from(&ciphertext_bytes[..]);
                js_sys::Reflect::set(&result, &"encryptedVote".into(), &ciphertext_array.into())?;

                // Parse and set inputs JSON
                match js_sys::JSON::parse(&inputs_json) {
                    Ok(js_value) => {
                        js_sys::Reflect::set(&result, &"inputs".into(), &js_value)?;
                        Ok(result.into())
                    }
                    Err(_) => Err(JsValue::from_str("Failed to parse inputs JSON")),
                }
            }
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
    }

    /// Generate CRISP ZK inputs for a vote update (either from voter or as a masker) from JavaScript.
    #[wasm_bindgen(js_name = "generateInputsForUpdate")]
    pub fn generate_inputs_for_update(
        &self,
        prev_ciphertext: &[u8],
        public_key: &[u8],
        vote: Vec<i64>,
    ) -> Result<JsValue, JsValue> {
        let vote_vec: Vec<u64> = vote.into_iter().map(|v| v as u64).collect();

        match self
            .generator
            .generate_inputs_for_update(prev_ciphertext, public_key, vote_vec)
        {
            Ok((ciphertext_bytes, inputs_json)) => {
                // Parse the JSON string and return as an object with both encryptedVote and inputs.
                let result = js_sys::Object::new();

                // Set encryptedVote as Uint8Array
                let ciphertext_array = js_sys::Uint8Array::from(&ciphertext_bytes[..]);
                js_sys::Reflect::set(&result, &"encryptedVote".into(), &ciphertext_array.into())?;

                // Parse and set inputs JSON
                match js_sys::JSON::parse(&inputs_json) {
                    Ok(js_value) => {
                        js_sys::Reflect::set(&result, &"inputs".into(), &js_value)?;
                        Ok(result.into())
                    }
                    Err(_) => Err(JsValue::from_str("Failed to parse inputs JSON")),
                }
            }
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
    }

    /// Generate a public key from JavaScript.
    #[wasm_bindgen(js_name = "generatePublicKey")]
    pub fn generate_public_key(&self) -> Result<Vec<u8>, JsValue> {
        match self.generator.generate_public_key() {
            Ok(public_key_bytes) => Ok(public_key_bytes),
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
    }

    /// Compute the commitment to a set of ciphertext polynomials from JavaScript.
    #[wasm_bindgen(js_name = "computeCiphertextCommitment")]
    pub fn compute_ciphertext_commitment(
        &self,
        ct0is: JsValue,
        ct1is: JsValue,
    ) -> Result<String, JsValue> {
        // Parse nested arrays: ct0is and ct1is are arrays of arrays (one array per CRT limb)
        let ct0is_array: js_sys::Array = js_sys::Array::from(&ct0is);
        let ct1is_array: js_sys::Array = js_sys::Array::from(&ct1is);

        let mut ct0is_vec: Vec<Vec<BigInt>> = Vec::new();
        for i in 0..ct0is_array.length() {
            let inner_array = ct0is_array
                .get(i)
                .dyn_into::<js_sys::Array>()
                .map_err(|_| JsValue::from_str("Expected array of arrays for ct0is"))?;

            let mut coefficients: Vec<BigInt> = Vec::new();
            for j in 0..inner_array.length() {
                let s = inner_array
                    .get(j)
                    .as_string()
                    .ok_or_else(|| JsValue::from_str("Expected string in inner array"))?;
                let bigint = s
                    .parse::<BigInt>()
                    .map_err(|e| JsValue::from_str(&format!("Failed to parse BigInt: {}", e)))?;
                coefficients.push(bigint);
            }
            ct0is_vec.push(coefficients);
        }

        let mut ct1is_vec: Vec<Vec<BigInt>> = Vec::new();
        for i in 0..ct1is_array.length() {
            let inner_array = ct1is_array
                .get(i)
                .dyn_into::<js_sys::Array>()
                .map_err(|_| JsValue::from_str("Expected array of arrays for ct1is"))?;

            let mut coefficients: Vec<BigInt> = Vec::new();
            for j in 0..inner_array.length() {
                let s = inner_array
                    .get(j)
                    .as_string()
                    .ok_or_else(|| JsValue::from_str("Expected string in inner array"))?;
                let bigint = s
                    .parse::<BigInt>()
                    .map_err(|e| JsValue::from_str(&format!("Failed to parse BigInt: {}", e)))?;
                coefficients.push(bigint);
            }
            ct1is_vec.push(coefficients);
        }

        let ct0 = CrtPolynomial::from_bigint_vectors(ct0is_vec);
        let ct1 = CrtPolynomial::from_bigint_vectors(ct1is_vec);

        Ok(self
            .generator
            .compute_ciphertext_commitment(&ct0, &ct1)
            .to_string())
    }

    /// Encrypt a vote from JavaScript.
    #[wasm_bindgen(js_name = "encryptVote")]
    pub fn encrypt_vote(&self, public_key: &[u8], vote: Vec<i64>) -> Result<Vec<u8>, JsValue> {
        let vote_vec: Vec<u64> = vote.into_iter().map(|v| v as u64).collect();

        match self.generator.encrypt_vote(public_key, vote_vec) {
            Ok(ciphertext_bytes) => Ok(ciphertext_bytes),
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
    }

    /// Get the BFV parameters used by the generator.
    #[wasm_bindgen(js_name = "getBFVParams")]
    pub fn get_bfv_params(&self) -> Result<JsValue, JsValue> {
        let bfv_params = self.generator.get_bfv_params();
        let params_json = js_sys::Object::new();

        // Set degree
        js_sys::Reflect::set(
            &params_json,
            &"degree".into(),
            &JsValue::from(bfv_params.degree() as u32),
        )?;

        // Set plaintext_modulus as BigInt to preserve precision for large values.
        let plaintext_modulus_bigint =
            js_sys::BigInt::new(&JsValue::from_str(&bfv_params.plaintext().to_string()))?;
        js_sys::Reflect::set(
            &params_json,
            &"plaintextModulus".into(),
            &plaintext_modulus_bigint.into(),
        )?;

        // Return moduli as array of BigInts to preserve precision
        let moduli_array = js_sys::Array::new();
        for modulus in bfv_params.moduli() {
            let modulus_bigint = js_sys::BigInt::new(&JsValue::from_str(&modulus.to_string()))?;
            moduli_array.push(&modulus_bigint.into());
        }
        js_sys::Reflect::set(&params_json, &"moduli".into(), &moduli_array.into())?;

        Ok(JsValue::from(params_json))
    }

    /// Get the version of the library.
    #[wasm_bindgen]
    pub fn version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_fhe_params::constants::insecure_512;
    use wasm_bindgen_test::*;
    const DEFAULT_DEGREE: usize = insecure_512::DEGREE;

    wasm_bindgen_test_configure!(run_in_browser);

    /// Helper function to create a vote vector with alternating 0s and 1s (deterministic).
    fn create_vote_vector() -> Vec<i64> {
        (0..DEFAULT_DEGREE).map(|i| (i % 2) as i64).collect()
    }

    #[wasm_bindgen_test]
    fn test_js_inputs_generation_with_defaults() {
        // Create generator with default parameters.
        let generator = ZKInputsGenerator::with_defaults().unwrap();
        let public_key = generator.generate_public_key().unwrap();
        let vote = create_vote_vector();
        let old_ciphertext = generator.encrypt_vote(&public_key, vote.clone()).unwrap();
        let result = generator.generate_inputs(&old_ciphertext, &public_key, vote);

        assert!(result.is_ok());

        let result_obj = result.unwrap();
        // Extract encryptedVote and inputs from the result object
        let encrypted_vote = js_sys::Reflect::get(&result_obj, &"encryptedVote".into()).unwrap();
        let inputs = js_sys::Reflect::get(&result_obj, &"inputs".into()).unwrap();

        // Verify encryptedVote is a Uint8Array and not empty
        assert!(encrypted_vote.is_object());
        let encrypted_vote_array = encrypted_vote.dyn_into::<js_sys::Uint8Array>().unwrap();
        assert!(encrypted_vote_array.length() > 0);

        // Convert JsValue to string for testing.
        let inputs_str = js_sys::JSON::stringify(&inputs)
            .unwrap()
            .as_string()
            .unwrap();
        assert!(inputs_str.contains("params"));
        assert!(inputs_str.contains("pk0is"));
    }

    #[wasm_bindgen_test]
    fn test_js_with_custom_params() {
        let degree = insecure_512::DEGREE;
        let plaintext_modulus = insecure_512::threshold::PLAINTEXT_MODULUS as i64;
        let moduli = insecure_512::threshold::MODULI
            .iter()
            .map(|m| *m as i64)
            .collect();

        // Create generator with custom parameters.
        let generator = ZKInputsGenerator::new(degree, plaintext_modulus, moduli).unwrap();
        let public_key = generator.generate_public_key().unwrap();
        let vote = create_vote_vector();
        let old_ciphertext = generator.encrypt_vote(&public_key, vote.clone()).unwrap();
        let result = generator.generate_inputs(&old_ciphertext, &public_key, vote);

        assert!(result.is_ok());

        let result_obj = result.unwrap();
        // Extract encryptedVote and inputs from the result object
        let encrypted_vote = js_sys::Reflect::get(&result_obj, &"encryptedVote".into()).unwrap();
        let inputs = js_sys::Reflect::get(&result_obj, &"inputs".into()).unwrap();

        // Verify encryptedVote is a Uint8Array and not empty
        assert!(encrypted_vote.is_object());
        let encrypted_vote_array = encrypted_vote.dyn_into::<js_sys::Uint8Array>().unwrap();
        assert!(encrypted_vote_array.length() > 0);

        // Convert JsValue to string for testing.
        let inputs_str = js_sys::JSON::stringify(&inputs)
            .unwrap()
            .as_string()
            .unwrap();
        assert!(inputs_str.contains("params"));
        assert!(inputs_str.contains("pk0is"));
    }
}
