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

use crate::{CorrelationId, E3id};

/// The compute instruction for a threadpool computation.
/// This enum provides protocol disambiguation
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
// #[rtype(result = "Result<ComputeResponse,ComputeRequestError>")]
#[rtype(result = "()")]
pub struct ComputeRequest {
    // TODO: Disambiguate protocol later
    pub request: e3_trbfv::TrBFVRequest,
    pub correlation_id: CorrelationId,
    pub e3_id: E3id, // It may come to pass this should be option
                     // but our initial need is only within the e3 flow
}
impl ComputeRequest {
    pub fn new(
        request: e3_trbfv::TrBFVRequest,
        correlation_id: CorrelationId,
        e3_id: E3id,
    ) -> Self {
        Self {
            request,
            correlation_id,
            e3_id,
        }
    }
}
impl ToString for ComputeRequest {
    fn to_string(&self) -> String {
        match self.request {
            e3_trbfv::TrBFVRequest::GenEsiSss(_) => "GenEsiSss",
            e3_trbfv::TrBFVRequest::GenPkShareAndSkSss(_) => "GenPkShareAndSkSss",
            e3_trbfv::TrBFVRequest::CalculateDecryptionKey(_) => "CalculateDecryptionKey",
            e3_trbfv::TrBFVRequest::CalculateDecryptionShare(_) => "CalculateDecryptionShare",
            e3_trbfv::TrBFVRequest::CalculateThresholdDecryption(_) => {
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
pub struct ComputeResponse {
    pub response: e3_trbfv::TrBFVResponse,
    pub correlation_id: CorrelationId,
    pub e3_id: E3id,
}

impl ComputeResponse {
    pub fn new(
        response: e3_trbfv::TrBFVResponse,
        correlation_id: CorrelationId,
        e3_id: E3id,
    ) -> ComputeResponse {
        ComputeResponse {
            response,
            correlation_id,
            e3_id,
        }
    }
}

/// An error from a threadpool computation
/// This enum provides protocol disambiguation
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ComputeRequestError {
    kind: ComputeRequestErrorKind,
    request: ComputeRequest,
}

impl ComputeRequestError {
    pub fn new(kind: ComputeRequestErrorKind, request: ComputeRequest) -> Self {
        Self { kind, request }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeRequestErrorKind {
    TrBFV(e3_trbfv::TrBFVError),
}

impl ComputeRequestError {
    pub fn get_err(&self) -> &ComputeRequestErrorKind {
        &self.kind
    }
}

impl std::error::Error for ComputeRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.get_err() {
            ComputeRequestErrorKind::TrBFV(err) => Some(err),
        }
    }
}

impl fmt::Display for ComputeRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get_err() {
            ComputeRequestErrorKind::TrBFV(err) => {
                write!(f, "We had an error number crunching: {:?}", err)
            }
        }
    }
}

impl TryFrom<ComputeResponse> for CalculateDecryptionShareResponse {
    type Error = anyhow::Error;
    fn try_from(value: ComputeResponse) -> Result<Self, Self::Error> {
        match value.response {
            TrBFVResponse::CalculateDecryptionShare(data) => Ok(data),
            _ => {
                bail!("Expected CalculateDecryptionShareResponse in response but it was not found")
            }
        }
    }
}

impl TryFrom<ComputeResponse> for CalculateDecryptionKeyResponse {
    type Error = anyhow::Error;
    fn try_from(value: ComputeResponse) -> Result<Self, Self::Error> {
        match value.response {
            TrBFVResponse::CalculateDecryptionKey(data) => Ok(data),
            _ => {
                bail!("Expected CalculateDecryptionKeyResponse in response but it was not found")
            }
        }
    }
}

impl TryFrom<ComputeResponse> for GenPkShareAndSkSssResponse {
    type Error = anyhow::Error;
    fn try_from(value: ComputeResponse) -> Result<Self, Self::Error> {
        match value.response {
            TrBFVResponse::GenPkShareAndSkSss(data) => Ok(data),
            _ => {
                bail!("Expected GenPkShareAndSkSssResponse in response but it was not found")
            }
        }
    }
}

impl TryFrom<ComputeResponse> for GenEsiSssResponse {
    type Error = anyhow::Error;
    fn try_from(value: ComputeResponse) -> Result<Self, Self::Error> {
        match value.response {
            TrBFVResponse::GenEsiSss(data) => Ok(data),
            _ => {
                bail!("Expected GenEsiSssResponse in response but it was not found")
            }
        }
    }
}

impl TryFrom<ComputeResponse> for CalculateThresholdDecryptionResponse {
    type Error = anyhow::Error;
    fn try_from(value: ComputeResponse) -> Result<Self, Self::Error> {
        match value.response {
            TrBFVResponse::CalculateThresholdDecryption(data) => Ok(data),
            _ => {
                bail!("Expected CalculateThresholdDecryptionResponse in response but it was not found")
            }
        }
    }
}
