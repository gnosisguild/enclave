// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! JavaScript library with WASM bindings for CRISP ZK inputs generation.
//!
//! This crate provides JavaScript bindings for the CRISP ZK inputs generator using WASM.

use js_sys;
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

        let generator = CoreZKInputsGenerator::new(degree, plaintext_modulus_u64, &moduli_vec)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(ZKInputsGenerator { generator })
    }

    /// Create a new JavaScript CRISP ZK inputs generator with default BFV parameters.
    #[wasm_bindgen(js_name = "withDefaults")]
    pub fn with_defaults() -> Result<ZKInputsGenerator, JsValue> {
        let generator = CoreZKInputsGenerator::with_defaults()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(ZKInputsGenerator { generator })
    }

    /// Generate a CRISP ZK inputs from JavaScript.
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
            Ok(inputs_json) => {
                // Parse the JSON string and return as JsValue.
                match js_sys::JSON::parse(&inputs_json) {
                    Ok(js_value) => Ok(js_value),
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
    use wasm_bindgen_test::*;
    use zk_inputs::DEFAULT_DEGREE;

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

        let inputs = result.unwrap();

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
        let degree = 2048;
        let plaintext_modulus = 1032193i64;
        let moduli = vec![0x3FFFFFFF000001i64];

        // Create generator with custom parameters.
        let generator = ZKInputsGenerator::new(degree, plaintext_modulus, moduli).unwrap();
        let public_key = generator.generate_public_key().unwrap();
        let vote = create_vote_vector();
        let old_ciphertext = generator.encrypt_vote(&public_key, vote.clone()).unwrap();
        let result = generator.generate_inputs(&old_ciphertext, &public_key, vote);

        assert!(result.is_ok());

        let inputs = result.unwrap();

        // Convert JsValue to string for testing.
        let inputs_str = js_sys::JSON::stringify(&inputs)
            .unwrap()
            .as_string()
            .unwrap();
        assert!(inputs_str.contains("params"));
        assert!(inputs_str.contains("pk0is"));
    }
}
