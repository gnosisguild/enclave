// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::Proof;
use derivative::Derivative;
use e3_crypto::SensitiveBytes;
use e3_fhe_params::BfvPreset;
use e3_utils::utility_types::ArcBytes;
use e3_zk_helpers::{computation::DkgInputType, CiphernodesCommitteeSize};
use serde::{Deserialize, Serialize};

/// ZK proof generation request variants.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZkRequest {
    /// Generate proof for BFV public key (C0).
    PkBfv(PkBfvProofRequest),
    /// Generate proof for PK generation (T1).
    PkGeneration(PkGenerationProofRequest),
    /// Generate proof for share and esm computation (T2a and T2b).
    ShareComputation(ShareComputationProofRequest),
    /// Generate proof for share encryption (C3a/C3b).
    ShareEncryption(ShareEncryptionProofRequest),
}

/// Request to generate a proof for share computation (T2a or T2b).
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct ShareComputationProofRequest {
    /// Raw secret polynomial bytes (sk or e_sm — witness, encrypted at rest).
    pub secret_raw: SensitiveBytes,
    /// Bincode-serialized SharedSecret containing Shamir shares (witness, encrypted at rest).
    pub secret_sss_raw: SensitiveBytes,
    /// Which secret type (SecretKey or SmudgingNoise).
    pub dkg_input_type: DkgInputType,
    /// BFV preset for parameter resolution.
    pub params_preset: BfvPreset,
    /// The size of the committee.
    pub committee_size: CiphernodesCommitteeSize,
}

/// Request to generate a proof for share encryption (C3a or C3b).
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct ShareEncryptionProofRequest {
    /// Bincode-serialized Vec<u64> share row coefficients.
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub share_row_raw: ArcBytes,
    /// Serialized BFV Ciphertext bytes (via fhe_traits::Serialize).
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub ciphertext_raw: ArcBytes,
    /// Serialized recipient BFV PublicKey bytes.
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub recipient_pk_raw: ArcBytes,
    /// Serialized u_rns Poly bytes.
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub u_rns_raw: ArcBytes,
    /// Serialized e0_rns Poly bytes.
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub e0_rns_raw: ArcBytes,
    /// Serialized e1_rns Poly bytes.
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub e1_rns_raw: ArcBytes,
    /// SecretKey or SmudgingNoise.
    pub dkg_input_type: DkgInputType,
    /// Threshold BFV preset (handler derives DKG params via build_pair_for_preset).
    pub params_preset: BfvPreset,
    /// Committee size.
    pub committee_size: CiphernodesCommitteeSize,
    /// Recipient index (for correlation tracking).
    pub recipient_party_id: usize,
    /// Modulus row index (for correlation tracking).
    pub row_index: usize,
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

/// Request to generate a proof for PK share generation (C1).
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct PkGenerationProofRequest {
    /// Raw pk0 share polynomial bytes (public statement).
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub pk0_share: ArcBytes,
    /// Raw secret key polynomial bytes (witness — encrypted at rest).
    pub sk: SensitiveBytes,
    /// Raw error polynomial bytes (witness — encrypted at rest).
    pub eek: SensitiveBytes,
    /// Raw smudging noise polynomial bytes (witness — encrypted at rest).
    pub e_sm: SensitiveBytes,
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
        sk: SensitiveBytes,
        eek: SensitiveBytes,
        e_sm: SensitiveBytes,
        params_preset: BfvPreset,
        committee_size: CiphernodesCommitteeSize,
    ) -> Self {
        Self {
            pk0_share: pk0_share.into(),
            sk,
            eek,
            params_preset,
            e_sm,
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
    /// Proof for share and esm computation (T2a and T2b).
    ShareComputation(ShareComputationProofResponse),
    /// Proof for share encryption (C3a/C3b).
    ShareEncryption(ShareEncryptionProofResponse),
}

/// Response containing a generated share computation proof.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShareComputationProofResponse {
    pub proof: Proof,
    pub dkg_input_type: DkgInputType,
}

/// Response containing a generated share encryption proof.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShareEncryptionProofResponse {
    pub proof: Proof,
    pub dkg_input_type: DkgInputType,
    pub recipient_party_id: usize,
    pub row_index: usize,
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

impl ShareComputationProofResponse {
    pub fn new(proof: Proof, dkg_input_type: DkgInputType) -> Self {
        Self {
            proof,
            dkg_input_type,
        }
    }
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
