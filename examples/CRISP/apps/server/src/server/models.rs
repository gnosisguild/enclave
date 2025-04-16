use serde::{Deserialize, Serialize};
use crate::server::database::SledDB;

pub struct AppState {
    pub sled: SledDB,
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
    pub address: String,
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

#[derive(Debug, Deserialize)]
pub struct CronRequestE3 {
    pub cron_api_key: String,
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
  
    pub committee_public_key: Vec<u8>,
    pub emojis: [String; 2],
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

impl From<E3> for E3StateLite {
    fn from(e3: E3) -> Self {
        E3StateLite {
            id: e3.id,
            chain_id: e3.chain_id,
            enclave_address: e3.enclave_address,
            status: e3.status,
            vote_count: e3.vote_count,
            start_time: e3.start_time,
            duration: e3.duration,
            expiration: e3.expiration,
            committee_public_key: e3.committee_public_key,
            emojis: e3.emojis,
        }
    }
}