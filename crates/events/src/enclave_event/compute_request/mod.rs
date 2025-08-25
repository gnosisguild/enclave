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

/// The compute instruction for a threadpool computation.
/// This enum provides protocol disambiguation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeRequest {
    /// By Protocol
    TrBFV(trbfv::TrBFVRequest),
    // Eg. TFHE(TFHERequest)
}

/// The compute result from a threadpool computation
/// This enum provides protocol disambiguation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeResponse {
    /// By Protocol
    TrBFV(trbfv::TrBFVResponse),
    // Eg. TFHE(TFHEResponse)
}

/// An error from a threadpool computation
/// This enum provides protocol disambiguation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeRequestError {
    /// By Protocol
    TrBFV(trbfv::TrBFVError),
    // Eg. TFHE(TFHEError)
}

// Actix messages
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<()>")]
pub struct ComputeRequested {
    pub correlation_id: CorrelationId,
    pub request: ComputeRequest,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ComputeRequestFailed {
    pub correlation_id: CorrelationId,
    pub request: ComputeRequest,
    pub error: ComputeRequestError,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ComputeRequestSucceeded {
    pub correlation_id: CorrelationId,
    pub response: ComputeResponse,
}
