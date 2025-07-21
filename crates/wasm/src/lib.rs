// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_bfv_helpers::client::bfv_encrypt_u64;
use rand::thread_rng;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn encrypt_number(data: u64, public_key: Vec<u8>) -> Result<Vec<u8>, JsValue> {
    let encrypted_data = bfv_encrypt_u64(data, public_key, thread_rng())
        .map_err(|e| JsValue::from_str(&format!("{}", e)))?;
    Ok(encrypted_data)
}
