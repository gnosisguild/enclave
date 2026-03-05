// SPDX-License-Identifier: LGPL-3.0-only

use derivative::Derivative;
use e3_utils::utility_types::ArcBytes;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A zero-knowledge proof with all data needed for verification.
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct Proof {
    /// Circuit that generated this proof.
    pub circuit: CircuitName,
    /// The proof bytes.
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub data: ArcBytes,
    /// Public signals from the circuit (inputs and outputs).
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub public_signals: ArcBytes,
}

impl Proof {
    pub fn new(
        circuit: CircuitName,
        data: impl Into<ArcBytes>,
        public_signals: impl Into<ArcBytes>,
    ) -> Self {
        Self {
            circuit,
            data: data.into(),
            public_signals: public_signals.into(),
        }
    }
}

/// Circuit variants determine the hash oracle used for VK generation and proving.
///
/// - `Default`: poseidon/`noir-recursive-no-zk` — wrapper & fold proofs (no ZK blinding, efficient).
/// - `Recursive`: poseidon/`noir-recursive` — inner/base proofs fed into a wrapper (ZK blinding preserved).
/// - `Evm`: keccak/`evm` — on-chain EVM-verifiable proofs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CircuitVariant {
    /// noir-recursive-no-zk: for wrapper & fold proofs — poseidon, no ZK blinding.
    #[default]
    Default,
    /// noir-recursive: for inner/base proofs — poseidon with ZK blinding.
    Recursive,
    /// evm: keccak-based for on-chain Solidity verification.
    Evm,
}

impl CircuitVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            CircuitVariant::Default => "default",
            CircuitVariant::Recursive => "recursive",
            CircuitVariant::Evm => "evm",
        }
    }

    /// Returns the bb verifier target flag value for this variant.
    pub fn verifier_target(&self) -> &'static str {
        match self {
            CircuitVariant::Default => "noir-recursive-no-zk",
            CircuitVariant::Recursive => "noir-recursive",
            CircuitVariant::Evm => "evm",
        }
    }
}

impl fmt::Display for CircuitVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Circuit identifiers for ZK proofs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CircuitName {
    /// BFV public key proof (C0).
    PkBfv,
    /// TrBFV public key share proof (C1).
    PkGeneration,
    /// Sk Share computation proof (C2a).
    SkShareComputation,
    /// E_SM share computation proof (C2b).
    ESmShareComputation,
    /// Share encryption proof (C3).
    ShareEncryption,
    /// DKG share decryption proof (C4).
    DkgShareDecryption,
    /// Public key aggregation proof (C5).
    PkAggregation,
    /// Decryption share proof (C6).
    ThresholdShareDecryption,
    /// Decrypted shares aggregation proof — BigNum variant (C7a).
    DecryptedSharesAggregationBn,
    /// Decrypted shares aggregation proof — Modular variant (C7b).
    DecryptedSharesAggregationMod,
    /// Recursive aggregation fold circuit (independent; lives at recursive_aggregation/fold).
    Fold,
}

impl CircuitName {
    pub fn as_str(&self) -> &'static str {
        match self {
            CircuitName::PkBfv => "pk",
            CircuitName::PkGeneration => "pk_generation",
            CircuitName::SkShareComputation => "sk_share_computation",
            CircuitName::ESmShareComputation => "e_sm_share_computation",
            CircuitName::ShareEncryption => "share_encryption",
            CircuitName::DkgShareDecryption => "share_decryption",
            CircuitName::PkAggregation => "pk_aggregation",
            CircuitName::ThresholdShareDecryption => "share_decryption",
            CircuitName::DecryptedSharesAggregationBn => "decrypted_shares_aggregation_bn",
            CircuitName::DecryptedSharesAggregationMod => "decrypted_shares_aggregation_mod",
            CircuitName::Fold => "fold",
        }
    }

    pub fn group(&self) -> &'static str {
        match self {
            CircuitName::PkBfv => "dkg",
            CircuitName::SkShareComputation => "dkg",
            CircuitName::ESmShareComputation => "dkg",
            CircuitName::ShareEncryption => "dkg",
            CircuitName::DkgShareDecryption => "dkg",
            CircuitName::PkGeneration => "threshold",
            CircuitName::ThresholdShareDecryption => "threshold",
            CircuitName::PkAggregation => "threshold",
            CircuitName::DecryptedSharesAggregationBn => "threshold",
            CircuitName::DecryptedSharesAggregationMod => "threshold",
            CircuitName::Fold => "recursive_aggregation",
        }
    }

    pub fn dir_path(&self) -> String {
        format!("{}/{}", self.group(), self.as_str())
    }

    /// Wrapper circuit path: `recursive_aggregation/wrapper/{group}/{name}`.
    pub fn wrapper_dir_path(&self) -> String {
        format!("recursive_aggregation/wrapper/{}", self.dir_path())
    }
}

impl fmt::Display for CircuitName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.dir_path())
    }
}
