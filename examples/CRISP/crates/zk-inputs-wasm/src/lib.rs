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
    /// - `plaintext_modulus`: Plaintext modulus
    /// - `moduli`: Array of moduli
    #[wasm_bindgen(constructor)]
    pub fn new(
        degree: usize,
        plaintext_modulus: u64,
        moduli: Vec<u64>,
    ) -> Result<ZKInputsGenerator, JsValue> {
        let generator = CoreZKInputsGenerator::new(degree, plaintext_modulus, &moduli)
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
        vote: Vec<u64>,
    ) -> Result<JsValue, JsValue> {
        match self
            .generator
            .generate_inputs(prev_ciphertext, public_key, vote)
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
    pub fn encrypt_vote(&self, public_key: &[u8], vote: Vec<u64>) -> Result<Vec<u8>, JsValue> {
        match self.generator.encrypt_vote(public_key, vote) {
            Ok(ciphertext_bytes) => Ok(ciphertext_bytes),
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
    }

    /// Get the BFV parameters used by the generator.
    #[wasm_bindgen(js_name = "getBFVParams")]
    pub fn get_bfv_params(&self) -> Result<JsValue, JsValue> {
        let bfv_params = self.generator.get_bfv_params();
        let params_json = js_sys::Object::new();
        js_sys::Reflect::set(
            &params_json,
            &"degree".into(),
            &JsValue::from(bfv_params.degree() as u32),
        )?;
        js_sys::Reflect::set(
            &params_json,
            &"plaintext_modulus".into(),
            &JsValue::from(bfv_params.plaintext() as f64),
        )?;
        let moduli_array = js_sys::Array::new();
        for modulus in bfv_params.moduli() {
            moduli_array.push(&JsValue::from(*modulus as f64));
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
    fn create_vote_vector() -> Vec<u64> {
        (0..DEFAULT_DEGREE).map(|i| (i % 2) as u64).collect()
    }

    #[wasm_bindgen_test]
    fn test_js_inputs_generation_with_defaults() {
        // Create generator with default parameters.
        let generator = ZKInputsGenerator::with_defaults().unwrap();
        let public_key = generator.generate_public_key().unwrap();
        let vote = create_vote_vector();
        let old_ciphertext = generator.encrypt_vote(&public_key, vote.clone()).unwrap();
        let result = generator.generate_inputs(&old_ciphertext, &public_key, vote.clone());

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
        let plaintext_modulus = 1032193;
        let moduli = vec![0x3FFFFFFF000001];

        // Create generator with custom parameters.
        let generator = ZKInputsGenerator::new(degree, plaintext_modulus, moduli).unwrap();
        let public_key = generator.generate_public_key().unwrap();
        let vote = create_vote_vector();
        let old_ciphertext = generator.encrypt_vote(&public_key, vote.clone()).unwrap();
        let result = generator.generate_inputs(&old_ciphertext, &public_key, vote.clone());

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
