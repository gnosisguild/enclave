//! JavaScript library with WASM bindings for CRISP inputs generation.
//!
//! This crate provides JavaScript bindings for the CRISP inputs generator using WASM.

use crisp_zk_inputs::CrispZKInputsGenerator;
use js_sys;
use wasm_bindgen::prelude::*;

/// JavaScript-compatible CRISP inputs generator.
#[wasm_bindgen]
pub struct ZKInputsGenerator {
    generator: CrispZKInputsGenerator,
}

#[wasm_bindgen]
impl ZKInputsGenerator {
    /// Create a new JavaScript CRISP inputs generator.
    #[wasm_bindgen(constructor)]
    pub fn new() -> ZKInputsGenerator {
        ZKInputsGenerator {
            generator: CrispZKInputsGenerator::new(),
        }
    }

    /// Generate CRISP inputs from JavaScript.
    #[wasm_bindgen(js_name = "generateInputs")]
    pub fn generate_inputs(
        &self,
        old_ciphertext: &str,
        public_key: &str,
        vote: u8,
    ) -> Result<JsValue, JsValue> {
        match self
            .generator
            .generate_inputs(old_ciphertext, public_key, vote)
        {
            Ok(inputs_json) => {
                // Parse the JSON string and return as JsValue.
                match js_sys::JSON::parse(&inputs_json) {
                    Ok(js_value) => Ok(js_value),
                    Err(_) => Err(JsValue::from_str("Failed to parse inputs JSON")),
                }
            }
            Err(e) => Err(JsValue::from_str(&e)),
        }
    }

    /// Generate CRISP public key from JavaScript.
    #[wasm_bindgen(js_name = "generatePublicKey")]
    pub fn generate_public_key(&self) -> Result<String, JsValue> {
        match self.generator.generate_public_key() {
            Ok(public_key) => Ok(public_key),
            Err(e) => Err(JsValue::from_str(&e)),
        }
    }

    /// Encrypt CRISP vote from JavaScript.
    #[wasm_bindgen(js_name = "encryptVote")]
    pub fn encrypt_vote(&self, public_key: &str, vote: u8) -> Result<String, JsValue> {
        match self.generator.encrypt_vote(public_key, vote) {
            Ok(ciphertext) => Ok(ciphertext),
            Err(e) => Err(JsValue::from_str(&e)),
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
    fn test_js_inputs_generation() {
        let generator = ZKInputsGenerator::new();
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
