// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use serde::{Deserialize, Serialize};

// This correlates with the information from the contract
// with an addition of a chain_id
#[derive(Debug, Deserialize, Serialize)]
pub struct E3 {
    pub chain_id: u64,
    pub ciphertext_inputs: Vec<(Vec<u8>, u64)>,
    pub ciphertext_output: Vec<u8>,
    pub committee_public_key: Vec<u8>,
    pub duration: u64,
    pub e3_params: Vec<u8>,
    pub custom_params: Vec<u8>,
    pub enclave_address: String,
    pub encryption_scheme_id: Vec<u8>,
    pub expiration: u64,
    pub id: u64,
    pub plaintext_output: Vec<u8>,
    pub request_block: u64,
    pub seed: [u8; 32],
    pub start_window: [u64; 2],
    pub threshold: [u32; 2],
}
