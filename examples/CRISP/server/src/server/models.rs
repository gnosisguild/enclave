// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Deserialize, Debug)]
pub struct WebhookPayload {
    pub e3_id: u64,
    #[serde(deserialize_with = "deserialize_hex_string")]
    pub ciphertext: Vec<u8>,
    #[serde(deserialize_with = "deserialize_hex_string")]
    pub proof: Vec<u8>,
}

pub fn deserialize_hex_string<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let hex_str = s.strip_prefix("0x").unwrap_or(&s);
    hex::decode(hex_str).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonResponse {
    pub response: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VoteResponseStatus {
    Success,
    UserAlreadyVoted,
    FailedBroadcast,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VoteResponse {
    pub status: VoteResponseStatus,
    pub tx_hash: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RoundCount {
    pub round_count: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CurrentRound {
    pub id: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PKRequest {
    pub round_id: u64,
    pub pk_bytes: Vec<u8>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CTRequest {
    pub round_id: u64,
    pub ct_bytes: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EncryptedVote {
    pub round_id: u64,
    pub enc_vote_bytes: Vec<u8>,
    pub proof: Vec<u8>,
    pub public_inputs: Vec<[u8; 32]>,
    pub address: String,
    pub proof_sem: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetRoundRequest {
    pub round_id: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ComputeProviderParams {
    pub name: String,
    pub parallel: bool,
    pub batch_size: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CustomParams {
    pub token_address: String,
    pub balance_threshold: String,
}

#[derive(Debug, Deserialize)]
pub struct RoundRequest {
    pub cron_api_key: String,
    pub token_address: String,
    pub balance_threshold: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WebResultRequest {
    pub round_id: u64,
    pub option_1_tally: u64,
    pub option_2_tally: u64,
    pub total_votes: u64,
    pub option_1_emoji: String,
    pub option_2_emoji: String,
    pub end_time: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct E3StateLite {
    pub id: u64,
    pub chain_id: u64,
    pub enclave_address: String,

    pub status: String,
    pub vote_count: u64,

    pub start_time: u64,
    pub duration: u64,
    pub expiration: u64,
    pub start_block: u64,

    pub committee_public_key: Vec<u8>,
    pub emojis: [String; 2],

    pub token_address: String,
    pub balance_threshold: String,
}


#[derive(Debug, Deserialize, Serialize)]
pub struct E3 {
    // Identifiers
    pub id: u64,
    pub chain_id: u64,
    pub enclave_address: String,

    // Status-related
    pub status: String,
    pub has_voted: Vec<String>,
    pub vote_count: u64,
    pub votes_option_1: u64,
    pub votes_option_2: u64,

    // Timing-related
    pub start_time: u64,
    pub block_start: u64,
    pub duration: u64,
    pub expiration: u64,

    // Parameters
    pub e3_params: Vec<u8>,
    pub committee_public_key: Vec<u8>,

    // Outputs
    pub ciphertext_output: Vec<u8>,
    pub plaintext_output: Vec<u8>,

    // Ciphertext Inputs
    pub ciphertext_inputs: Vec<(Vec<u8>, u64)>,

    // Emojis
    pub emojis: [String; 2],

    // Custom Parameters
    pub custom_params: CustomParams,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct E3Crisp {
    pub emojis: [String; 2],
    pub has_voted: Vec<String>,
    pub start_time: u64,
    pub status: String,
    pub votes_option_1: u64,
    pub votes_option_2: u64,
    pub token_holder_hashes: Vec<String>,
    pub token_address: String,
    pub balance_threshold: String,
    pub block_number_requested: u64, 
}

impl From<E3> for WebResultRequest {
    fn from(e3: E3) -> Self {
        WebResultRequest {
            round_id: e3.id,
            option_1_tally: e3.votes_option_1,
            option_2_tally: e3.votes_option_2,
            total_votes: e3.votes_option_1 + e3.votes_option_2,
            option_1_emoji: e3.emojis[0].clone(),
            option_2_emoji: e3.emojis[1].clone(),
            end_time: e3.expiration,
        }
    }
}
