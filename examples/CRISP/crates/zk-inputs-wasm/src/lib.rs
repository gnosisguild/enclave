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
        let generator = CoreZKInputsGenerator::new(degree, plaintext_modulus, &moduli);
        Ok(ZKInputsGenerator { generator })
    }

    /// Create a new JavaScript CRISP ZK inputs generator with default BFV parameters.
    #[wasm_bindgen(js_name = "withDefaults")]
    pub fn with_defaults() -> ZKInputsGenerator {
        let generator = CoreZKInputsGenerator::with_defaults();
        ZKInputsGenerator { generator }
    }

    /// Generate a CRISP ZK inputs from JavaScript.
    #[wasm_bindgen(js_name = "generateInputs")]
    pub fn generate_inputs(
        &self,
        prev_ciphertext: &[u8],
        public_key: &[u8],
        vote: u8,
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
    pub fn encrypt_vote(&self, public_key: &[u8], vote: u8) -> Result<Vec<u8>, JsValue> {
        match self.generator.encrypt_vote(public_key, vote) {
            Ok(ciphertext_bytes) => Ok(ciphertext_bytes),
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
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

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_js_inputs_generation_with_defaults() {
        // Create generator with default parameters.
        let generator = ZKInputsGenerator::with_defaults();
        let public_key = generator.generate_public_key().unwrap();
        let old_ciphertext = generator.encrypt_vote(&public_key, 1).unwrap();
        let result = generator.generate_inputs(&old_ciphertext, &public_key, 1);

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
        let old_ciphertext = generator.encrypt_vote(&public_key, 1).unwrap();
        let result = generator.generate_inputs(&old_ciphertext, &public_key, 1);

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
