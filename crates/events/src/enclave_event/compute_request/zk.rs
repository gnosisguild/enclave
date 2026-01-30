// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::Proof;
use derivative::Derivative;
use e3_utils::utility_types::ArcBytes;
use serde::{Deserialize, Serialize};

/// ZK proof generation request variants.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZkRequest {
    /// Generate proof for BFV public key (T0).
    PkBfv(PkBfvProofRequest),
}

/// Request to generate a proof for BFV public key generation (T0).
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct PkBfvProofRequest {
    /// The BFV public key bytes.
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub pk_bfv: ArcBytes,
    /// ABI-encoded BFV parameters.
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub params: ArcBytes,
}

impl PkBfvProofRequest {
    pub fn new(pk_bfv: impl Into<ArcBytes>, params: impl Into<ArcBytes>) -> Self {
        Self {
            pk_bfv: pk_bfv.into(),
            params: params.into(),
        }
    }
}

/// ZK proof generation response variants.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZkResponse {
    /// Proof for BFV public key (T0).
    PkBfv(PkBfvProofResponse),
}

/// Response containing a generated BFV public key proof.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PkBfvProofResponse {
    pub proof: Proof,
}

impl PkBfvProofResponse {
    pub fn new(proof: Proof) -> Self {
        Self { proof }
    }
}

/// ZK-specific error variants.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZkError {
    /// Proof generation failed.
    ProofGenerationFailed(String),
    /// Witness generation failed.
    WitnessGenerationFailed(String),
    /// Invalid parameters.
    InvalidParams(String),
}

impl std::fmt::Display for ZkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZkError::ProofGenerationFailed(msg) => write!(f, "Proof generation failed: {}", msg),
            ZkError::WitnessGenerationFailed(msg) => {
                write!(f, "Witness generation failed: {}", msg)
            }
            ZkError::InvalidParams(msg) => write!(f, "Invalid parameters: {}", msg),
        }
    }
}

impl std::error::Error for ZkError {}
