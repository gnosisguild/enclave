// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_bfv_helpers::client::{bfv_encrypt, bfv_verifiable_encrypt};
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
    moduli: u64,
) -> Result<Vec<u8>, JsValue> {
    let encrypted_data = bfv_encrypt([data], public_key, degree, plaintext_modulus, [moduli])
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
    moduli: u64,
) -> Result<Vec<u8>, JsValue> {
    let encrypted_data = bfv_encrypt(data, public_key, degree, plaintext_modulus, [moduli])
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
    moduli: u64,
) -> Result<Vec<JsValue>, JsValue> {
    let result = bfv_verifiable_encrypt([data], public_key, degree, plaintext_modulus, [moduli])
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
    moduli: u64,
) -> Result<Vec<JsValue>, JsValue> {
    let result = bfv_verifiable_encrypt(data, public_key, degree, plaintext_modulus, [moduli])
        .map_err(|e| JsValue::from_str(&format!("{}", e)))?;

    // Return as a vector of JsValues
    Ok(vec![
        JsValue::from(result.encrypted_data),
        JsValue::from(result.circuit_inputs),
    ])
}
