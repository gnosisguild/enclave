// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::Proof;
use derivative::Derivative;
use e3_fhe_params::BfvPreset;
use e3_utils::utility_types::ArcBytes;
use e3_zk_helpers::CiphernodesCommitteeSize;
use serde::{Deserialize, Serialize};

/// ZK proof generation request variants.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZkRequest {
    /// Generate proof for BFV public key (T0).
    PkBfv(PkBfvProofRequest),
    /// Generate proof for PK generation (T1a).
    PkGeneration(PkGenerationProofRequest),
}

/// Request to generate a proof for BFV public key generation (T0).
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct PkBfvProofRequest {
    /// The BFV public key bytes.
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub pk_bfv: ArcBytes,
    pub params_preset: BfvPreset,
}

/// Request to generate a proof for PK share generation (T1a).
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct PkGenerationProofRequest {
    /// Raw pk0 share polynomial bytes.
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub pk0_share: ArcBytes,
    /// Raw common random polynomial bytes.
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub a: ArcBytes,
    /// Raw secret key polynomial bytes (witness).
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub sk: ArcBytes,
    /// Raw error polynomial bytes (witness).
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub eek: ArcBytes,
    /// Raw smudging noise polynomial bytes (witness).
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub e_sm: ArcBytes,
    /// BFV preset for parameter resolution.
    pub params_preset: BfvPreset,
    /// The size of the committee
    pub committee_size: CiphernodesCommitteeSize,
}

impl PkBfvProofRequest {
    pub fn new(pk_bfv: impl Into<ArcBytes>, params_preset: BfvPreset) -> Self {
        Self {
            pk_bfv: pk_bfv.into(),
            params_preset,
        }
    }
}

impl PkGenerationProofRequest {
    pub fn new(
        pk0_share: impl Into<ArcBytes>,
        a: impl Into<ArcBytes>,
        sk: impl Into<ArcBytes>,
        eek: impl Into<ArcBytes>,
        e_sm: impl Into<ArcBytes>,
        params_preset: BfvPreset,
        committee_size: CiphernodesCommitteeSize,
    ) -> Self {
        Self {
            pk0_share: pk0_share.into(),
            a: a.into(),
            sk: sk.into(),
            eek: eek.into(),
            params_preset,
            e_sm: e_sm.into(),
            committee_size,
        }
    }
}

/// ZK proof generation response variants.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZkResponse {
    /// Proof for BFV public key (T0).
    PkBfv(PkBfvProofResponse),
    /// Proof for PK generation (T1a).
    PkGeneration(PkGenerationProofResponse),
}

/// Response containing a generated BFV public key proof.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PkBfvProofResponse {
    pub proof: Proof,
}

/// Response containing a generated PK generation proof.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PkGenerationProofResponse {
    pub proof: Proof,
}

impl PkBfvProofResponse {
    pub fn new(proof: Proof) -> Self {
        Self { proof }
    }
}

impl PkGenerationProofResponse {
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
