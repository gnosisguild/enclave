// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use core::fmt;

use actix::Message;
use anyhow::bail;
use e3_trbfv::{
    calculate_decryption_key::CalculateDecryptionKeyResponse,
    calculate_decryption_share::CalculateDecryptionShareResponse,
    calculate_threshold_decryption::CalculateThresholdDecryptionResponse,
    gen_esi_sss::GenEsiSssResponse, gen_pk_share_and_sk_sss::GenPkShareAndSkSssResponse,
    TrBFVResponse,
};
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

impl ToString for ComputeRequest {
    fn to_string(&self) -> String {
        match self {
            Self::TrBFV(e3_trbfv::TrBFVRequest::GenEsiSss(_)) => "GenEsiSss",
            Self::TrBFV(e3_trbfv::TrBFVRequest::GenPkShareAndSkSss(_)) => "GenPkShareAndSkSss",
            Self::TrBFV(e3_trbfv::TrBFVRequest::CalculateDecryptionKey(_)) => {
                "CalculateDecryptionKey"
            }
            Self::TrBFV(e3_trbfv::TrBFVRequest::CalculateDecryptionShare(_)) => {
                "CalculateDecryptionShare"
            }
            Self::TrBFV(e3_trbfv::TrBFVRequest::CalculateThresholdDecryption(_)) => {
                "CalculateThresholdDecryption"
            }
        }
        .to_string()
    }
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
    RecvError(String),
    SemaphoreError(String),
    // Eg. TFHE(TFHEError)
}

impl std::error::Error for ComputeRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ComputeRequestError::TrBFV(err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for ComputeRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComputeRequestError::TrBFV(err) => {
                write!(f, "We had an error number crunching: {:?}", err)
            }
            ComputeRequestError::SemaphoreError(name) => {
                write!(f, "Multithread SemaphoreError. This means there was a problem acquiring the semaphore lock for this ComputeRequest: '{name}'")
            }
            ComputeRequestError::RecvError(name) => {
                write!(f, "Multithread RecvError. This means there was a problem acquiring the semaphore lock for this ComputeRequest: '{name}'")
            }
        }
    }
}

impl TryFrom<ComputeResponse> for CalculateDecryptionShareResponse {
    type Error = anyhow::Error;
    fn try_from(value: ComputeResponse) -> Result<Self, Self::Error> {
        match value {
            ComputeResponse::TrBFV(TrBFVResponse::CalculateDecryptionShare(data)) => Ok(data),
            _ => {
                bail!("Expected CalculateDecryptionShareResponse in response but it was not found")
            }
        }
    }
}

impl TryFrom<ComputeResponse> for CalculateDecryptionKeyResponse {
    type Error = anyhow::Error;
    fn try_from(value: ComputeResponse) -> Result<Self, Self::Error> {
        match value {
            ComputeResponse::TrBFV(TrBFVResponse::CalculateDecryptionKey(data)) => Ok(data),
            _ => {
                bail!("Expected CalculateDecryptionKeyResponse in response but it was not found")
            }
        }
    }
}

impl TryFrom<ComputeResponse> for GenPkShareAndSkSssResponse {
    type Error = anyhow::Error;
    fn try_from(value: ComputeResponse) -> Result<Self, Self::Error> {
        match value {
            ComputeResponse::TrBFV(TrBFVResponse::GenPkShareAndSkSss(data)) => Ok(data),
            _ => {
                bail!("Expected GenPkShareAndSkSssResponse in response but it was not found")
            }
        }
    }
}

impl TryFrom<ComputeResponse> for GenEsiSssResponse {
    type Error = anyhow::Error;
    fn try_from(value: ComputeResponse) -> Result<Self, Self::Error> {
        match value {
            ComputeResponse::TrBFV(TrBFVResponse::GenEsiSss(data)) => Ok(data),
            _ => {
                bail!("Expected GenEsiSssResponse in response but it was not found")
            }
        }
    }
}

impl TryFrom<ComputeResponse> for CalculateThresholdDecryptionResponse {
    type Error = anyhow::Error;
    fn try_from(value: ComputeResponse) -> Result<Self, Self::Error> {
        match value {
            ComputeResponse::TrBFV(TrBFVResponse::CalculateThresholdDecryption(data)) => Ok(data),
            _ => {
                bail!("Expected CalculateThresholdDecryptionResponse in response but it was not found")
            }
        }
    }
}
