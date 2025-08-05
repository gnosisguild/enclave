// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pub mod bytes;
pub mod trbfv;

use crate::CorrelationId;
use actix::Message;
use serde::{Deserialize, Serialize};

/// Input format for TrBFVRequest
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVRequest {
    GenerateEsiShares(trbfv::generate_esi_shares::Request),
    GenerateSecretShares(trbfv::generate_secret_shares::Request),
    GenerateDecryptionKey(trbfv::generate_decryption_key::Request),
    GenerateDecryptionShare(trbfv::generate_decryption_share::Request),
    ThresholdDecrypt(trbfv::threshold_decrypt::Request),
}

/// Result format for TrBFVResponse
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVResponse {
    GenerateEsiShares(trbfv::generate_esi_shares::Response),
    GenerateSecretShares(trbfv::generate_secret_shares::Response),
    GenerateDecryptionKey(trbfv::generate_decryption_key::Response),
    GenerateDecryptionShare(trbfv::generate_decryption_share::Response),
    ThresholdDecrypt(trbfv::threshold_decrypt::Response),
}

/// The compute instruction for a threadpool computation.
/// This enum provides protocol disambiguation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeRequest {
    /// By Protocol
    TrBFV(TrBFVRequest),
    // Eg. TFHE(TFHERequest)
}

/// The compute result from a threadpool computation
/// This enum provides protocol disambiguation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeResponse {
    /// By Protocol
    TrBFV(TrBFVResponse),
    // Eg. TFHE(TFHEResponse)
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeRequestError {}

// Actix messages
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ComputeRequested {
    correlation_id: CorrelationId,
    request: ComputeRequest,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ComputeRequestFailed {
    correlation_id: CorrelationId,
    request: ComputeRequest,
    error: ComputeRequestError,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ComputeRequestSucceeded {
    correlation_id: CorrelationId,
    response: ComputeResponse,
}
