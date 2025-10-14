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

impl std::error::Error for ComputeRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ComputeRequestError::TrBFV(err) => Some(err),
        }
    }
}

impl fmt::Display for ComputeRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComputeRequestError::TrBFV(err) => {
                write!(f, "TrBFV error: {:?}", err)
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
