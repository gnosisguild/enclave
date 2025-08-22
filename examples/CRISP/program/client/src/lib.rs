// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ComputeRequest {
    pub e3_id: u64,
    pub params: Vec<u8>,
    pub ciphertext_inputs: Vec<(Vec<u8>, u64)>,
    pub webhook_url: String,
}

#[derive(Deserialize, Serialize)]
pub struct ComputeResponse {
    pub status: String,
    pub e3_id: u64,
}

pub async fn run_compute(
    e3_id: u64,
    params: Vec<u8>,
    ciphertext_inputs: Vec<(Vec<u8>, u64)>,
    webhook_url: String,
) -> Result<(u64, String)> {
    let request = ComputeRequest {
        e3_id,
        webhook_url,
        params,
        ciphertext_inputs,
    };

    let response: ComputeResponse = reqwest::Client::new()
        .post("http://127.0.0.1:13151/run_compute")
        .json(&request)
        .send()
        .await?
        .json()
        .await?;

    Ok((response.e3_id, response.status))
}
