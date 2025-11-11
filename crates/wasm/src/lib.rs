// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_bfv_helpers::{
    client::{bfv_encrypt, bfv_verifiable_encrypt},
    BfvParamSet,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
/// A function to encrypt a u64 value using BFV and default params.
///
/// # Arguments
///
/// * `data` - The data to encrypt - must be a u64
/// * `public_key` - The public key to be used for encryption
/// * `degree` - Polynomial degree for BFV parameters
/// * `plaintext_modulus` - Plaintext modulus for BFV parameters
/// * `moduli` - Modulus for BFV parameters
///
/// # Returns
///
/// Returns a `Result<Vec<u8>, JsValue>` containing the encrypted data and any errors.
///
/// # Panics
///
/// Panics if the data cannot be encrypted
pub fn bfv_encrypt_number(
    data: u64,
    public_key: Vec<u8>,
    degree: usize,
    plaintext_modulus: u64,
    moduli: Vec<u64>,
) -> Result<Vec<u8>, JsValue> {
    let encrypted_data = bfv_encrypt([data], public_key, degree, plaintext_modulus, &moduli)
        .map_err(|e| JsValue::from_str(&format!("{}", e)))?;
    Ok(encrypted_data)
}

#[wasm_bindgen]
/// A function to encrypt a Vec<u64> value using BFV and default params.
///
/// # Arguments
///
/// * `data` - The data to encrypt - must be a Vec<u64>
/// * `public_key` - The public key to be used for encryption
/// * `degree` - Polynomial degree for BFV parameters
/// * `plaintext_modulus` - Plaintext modulus for BFV parameters
/// * `moduli` - Modulus for BFV parameters
///
/// # Returns
///
/// Returns a `Result<Vec<u8>, JsValue>` containing the encrypted data and any errors.
///
/// # Panics
///
/// Panics if the data cannot be encrypted
pub fn bfv_encrypt_vector(
    data: Vec<u64>,
    public_key: Vec<u8>,
    degree: usize,
    plaintext_modulus: u64,
    moduli: Vec<u64>,
) -> Result<Vec<u8>, JsValue> {
    let encrypted_data = bfv_encrypt(data, public_key, degree, plaintext_modulus, &moduli)
        .map_err(|e| JsValue::from_str(&format!("{}", e)))?;
    Ok(encrypted_data)
}

#[wasm_bindgen]
/// A function to encrypt a u64 value using BFV and default params and
/// generate circuit inputs for Greco
///
/// # Arguments
///
/// * `data` - The data to encrypt - must be a u64
/// * `public_key` - The public key to be used for encryption
/// * `degree` - Polynomial degree for BFV parameters
/// * `plaintext_modulus` - Plaintext modulus for BFV parameters
/// * `moduli` - Modulus for BFV parameters
///
/// # Returns
///
/// Returns a `Result<Vec<JsValue>, JsValue>` containing the encrypted data, circuit inputs and any errors.
///
/// # Panics
///
/// Panics if the data cannot be encrypted
pub fn bfv_verifiable_encrypt_number(
    data: u64,
    public_key: Vec<u8>,
    degree: usize,
    plaintext_modulus: u64,
    moduli: Vec<u64>,
) -> Result<Vec<JsValue>, JsValue> {
    let result = bfv_verifiable_encrypt([data], public_key, degree, plaintext_modulus, moduli)
        .map_err(|e| JsValue::from_str(&format!("{}", e)))?;

    // Return as a vector of JsValues
    Ok(vec![
        JsValue::from(result.encrypted_data),
        JsValue::from(result.circuit_inputs),
    ])
}

#[wasm_bindgen]
/// A function to encrypt a Vec<u64> value using BFV and default params and
/// generate circuit inputs for Greco
///
/// # Arguments
///
/// * `data` - The data to encrypt - must be a Vec<u64>
/// * `public_key` - The public key to be used for encryption
/// * `degree` - Polynomial degree for BFV parameters
/// * `plaintext_modulus` - Plaintext modulus for BFV parameters
/// * `moduli` - Modulus for BFV parameters
///
/// # Returns
///
/// Returns a `Result<Vec<JsValue>, JsValue>` containing the encrypted data, circuit inputs and any errors.
///
/// # Panics
///
/// Panics if the data cannot be encrypted
pub fn bfv_verifiable_encrypt_vector(
    data: Vec<u64>,
    public_key: Vec<u8>,
    degree: usize,
    plaintext_modulus: u64,
    moduli: Vec<u64>,
) -> Result<Vec<JsValue>, JsValue> {
    let result = bfv_verifiable_encrypt(data, public_key, degree, plaintext_modulus, moduli)
        .map_err(|e| JsValue::from_str(&format!("{}", e)))?;

    // Return as a vector of JsValues
    Ok(vec![
        JsValue::from(result.encrypted_data),
        JsValue::from(result.circuit_inputs),
    ])
}

#[wasm_bindgen]
/// Retrieves a BFV parameter set by name.
///
/// # Parameters
/// * `name` - Parameter set identifier (e.g., "SET_8192_1000_4")
///
/// # Returns
/// A JavaScript object with the following structure:
/// ```typescript
/// {
///   degree: number;              // Polynomial degree (e.g., 8192)
///   plaintext_modulus: number;   // Plaintext modulus value (e.g., 1000)
///   moduli: number[];            // Array of moduli
///   error2_variance: string | null; // Error variance as string or null
/// }
/// ```
///
/// # Errors
/// Returns error if the parameter set name is invalid or serialization fails.
pub fn get_bfv_params(name: &str) -> Result<JsValue, JsValue> {
    let params = BfvParamSet::get_params(name).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let js_params = BfvParamSetJs::from(&params);
    serde_wasm_bindgen::to_value(&js_params)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

#[wasm_bindgen]
/// Returns all available BFV parameter set identifiers.
///
/// # Returns
/// Array of parameter set names that can be passed to `get_bfv_params()`.
/// Includes both production-ready sets (e.g., "SET_8192_1000_4") and
/// insecure sets for testing (prefixed with "INSECURE_").
pub fn get_bfv_params_list() -> Vec<String> {
    BfvParamSet::get_params_list()
}

#[derive(Serialize, Deserialize)]
pub struct BfvParamSetJs {
    pub degree: usize,
    pub plaintext_modulus: u64,
    pub moduli: Vec<u64>,
    pub error2_variance: Option<String>,
}

impl From<&BfvParamSet> for BfvParamSetJs {
    fn from(params: &BfvParamSet) -> Self {
        BfvParamSetJs {
            degree: params.degree,
            plaintext_modulus: params.plaintext_modulus,
            moduli: params.moduli.to_vec(),
            error2_variance: params.error2_variance.map(|s| s.to_string()),
        }
    }
}
