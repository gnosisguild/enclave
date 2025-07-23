// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ComputeRequest {
    pub params: Vec<u8>,
    pub ciphertext_inputs: Vec<(Vec<u8>, u64)>,
}

#[derive(Deserialize, Serialize)]
pub struct ComputeResponse {
    pub ciphertext: Vec<u8>,
    pub proof: Vec<u8>,
}

pub async fn run_compute(
    params: Vec<u8>,
    ciphertext_inputs: Vec<(Vec<u8>, u64)>,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let request = ComputeRequest {
        params,
        ciphertext_inputs,
    };

    let response: ComputeResponse = reqwest::Client::new()
        .post("http://127.0.0.1:4001/run_compute")
        .json(&request)
        .send()
        .await?
        .json()
        .await?;

    Ok((response.proof, response.ciphertext))
}
