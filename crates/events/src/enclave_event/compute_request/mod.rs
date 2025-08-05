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
    GenEsiShares(trbfv::gen_esi_shares::Request),
    GenPkShareAndSkSss(trbfv::gen_pk_share_and_sk_sss::Request),
    GenDecryptionKey(trbfv::gen_decryption_key::Request),
    GenDecryptionShare(trbfv::gen_decryption_share::Request),
    ThresholdDecrypt(trbfv::threshold_decrypt::Request),
}

/// Result format for TrBFVResponse
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVResponse {
    GenEsiShares(trbfv::gen_esi_shares::Response),
    GenPkShareAndSkSss(trbfv::gen_pk_share_and_sk_sss::Response),
    GenDecryptionKey(trbfv::gen_decryption_key::Response),
    GenDecryptionShare(trbfv::gen_decryption_share::Response),
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
