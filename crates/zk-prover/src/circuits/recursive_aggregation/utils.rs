// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::error::ZkError;

const FIELD_SIZE: usize = 32;

pub fn bytes_to_field_strings(bytes: &[u8]) -> Result<Vec<String>, ZkError> {
    if bytes.len() % FIELD_SIZE != 0 {
        return Err(ZkError::InvalidInput(format!(
            "expected length multiple of {FIELD_SIZE}, got {}",
            bytes.len()
        )));
    }
    Ok(bytes
        .chunks(FIELD_SIZE)
        .map(|chunk| format!("0x{}", hex::encode(chunk)))
        .collect())
}
