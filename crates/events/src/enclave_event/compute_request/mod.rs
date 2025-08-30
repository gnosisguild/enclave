// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::CorrelationId;
use actix::Message;
use e3_trbfv::{TrBFVRequest, TrBFVResponse};
use serde::{Deserialize, Serialize};

use super::EnclaveEvent;

/// The compute instruction for a threadpool computation.
/// This enum provides protocol disambiguation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeRequest {
    /// By Protocol
    TrBFV(e3_trbfv::TrBFVRequest),
    // Eg. TFHE(TFHERequest)
}

/// The compute result from a threadpool computation
/// This enum provides protocol disambiguation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

// Actix messages
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<()>")]
pub struct ComputeRequested {
    pub correlation_id: CorrelationId,
    pub payload: ComputeRequest,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ComputeRequestFailed {
    pub correlation_id: CorrelationId,
    pub payload: ComputeRequest,
    pub error: String,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ComputeRequestSucceeded {
    pub correlation_id: CorrelationId,
    pub payload: ComputeResponse,
}

impl Into<EnclaveEvent> for e3_trbfv::gen_pk_share_and_sk_sss::Request {
    fn into(self) -> EnclaveEvent {
        EnclaveEvent::from(ComputeRequested {
            correlation_id: CorrelationId::new(),
            payload: ComputeRequest::TrBFV(TrBFVRequest::GenPkShareAndSkSss(self)),
        })
    }
}

impl Into<EnclaveEvent> for e3_trbfv::gen_esi_sss::Request {
    fn into(self) -> EnclaveEvent {
        EnclaveEvent::from(ComputeRequested {
            correlation_id: CorrelationId::new(),
            payload: ComputeRequest::TrBFV(TrBFVRequest::GenEsiSss(self)),
        })
    }
}

impl Into<EnclaveEvent> for e3_trbfv::calculate_decryption_key::Request {
    fn into(self) -> EnclaveEvent {
        EnclaveEvent::from(ComputeRequested {
            correlation_id: CorrelationId::new(),
            payload: ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionKey(self)),
        })
    }
}

impl Into<EnclaveEvent> for e3_trbfv::calculate_decryption_share::Request {
    fn into(self) -> EnclaveEvent {
        EnclaveEvent::from(ComputeRequested {
            correlation_id: CorrelationId::new(),
            payload: ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionShare(self)),
        })
    }
}

impl Into<EnclaveEvent> for e3_trbfv::calculate_threshold_decryption::Request {
    fn into(self) -> EnclaveEvent {
        EnclaveEvent::from(ComputeRequested {
            correlation_id: CorrelationId::new(),
            payload: ComputeRequest::TrBFV(TrBFVRequest::CalculateThresholdDecryption(self)),
        })
    }
}

impl Into<EnclaveEvent> for e3_trbfv::gen_pk_share_and_sk_sss::Response {
    fn into(self) -> EnclaveEvent {
        EnclaveEvent::from(ComputeRequestSucceeded {
            correlation_id: CorrelationId::new(),
            payload: ComputeResponse::TrBFV(TrBFVResponse::GenPkShareAndSkSss(self)),
        })
    }
}

impl Into<EnclaveEvent> for e3_trbfv::gen_esi_sss::Response {
    fn into(self) -> EnclaveEvent {
        EnclaveEvent::from(ComputeRequestSucceeded {
            correlation_id: CorrelationId::new(),
            payload: ComputeResponse::TrBFV(TrBFVResponse::GenEsiSss(self)),
        })
    }
}

impl Into<EnclaveEvent> for e3_trbfv::calculate_decryption_key::Response {
    fn into(self) -> EnclaveEvent {
        EnclaveEvent::from(ComputeRequestSucceeded {
            correlation_id: CorrelationId::new(),
            payload: ComputeResponse::TrBFV(TrBFVResponse::CalculateDecryptionKey(self)),
        })
    }
}

impl Into<EnclaveEvent> for e3_trbfv::calculate_decryption_share::Response {
    fn into(self) -> EnclaveEvent {
        EnclaveEvent::from(ComputeRequestSucceeded {
            correlation_id: CorrelationId::new(),
            payload: ComputeResponse::TrBFV(TrBFVResponse::CalculateDecryptionShare(self)),
        })
    }
}

impl Into<EnclaveEvent> for e3_trbfv::calculate_threshold_decryption::Response {
    fn into(self) -> EnclaveEvent {
        EnclaveEvent::from(ComputeRequestSucceeded {
            correlation_id: CorrelationId::new(),
            payload: ComputeResponse::TrBFV(TrBFVResponse::CalculateThresholdDecryption(self)),
        })
    }
}
