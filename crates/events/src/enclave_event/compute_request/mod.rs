// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Message;
use serde::{Deserialize, Serialize};

/// The compute instruction for a threadpool computation.
/// This enum provides protocol disambiguation
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "Result<ComputeResponse,ComputeRequestError>")]
pub enum ComputeRequest {
    /// By Protocol
    TrBFV(e3_trbfv::TrBFVRequest),
    // Eg. TFHE(TFHERequest)
}

/// The compute result from a threadpool computation
/// This enum provides protocol disambiguation
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub enum ComputeResponse {
    /// By Protocol
    TrBFV(e3_trbfv::TrBFVResponse),
    // Eg. TFHE(TFHEResponse)
}

/// An error from a threadpool computation
/// This enum provides protocol disambiguation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeRequestError {
    /// By Protocol
    TrBFV(e3_trbfv::TrBFVError),
    // Eg. TFHE(TFHEError)
}
