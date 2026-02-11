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

/// Circuit identifiers for ZK proofs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CircuitName {
    /// BFV public key proof (T0).
    PkBfv,
    /// TrBFV public key share proof (T1a).
    PkGeneration,
    /// Sk Share computation proof (T1b).
    SkShareComputation,
    /// E_SM share computation proof (T1c).
    ESmShareComputation,
    /// Encrypted sk share proof (T1d).
    SkShareEncryption,
    /// Encrypted E_SM share proof (T1e).
    ESmShareEncryption,
    /// Decryption share proof (T4/T5).
    DecShares,
    /// Public key aggregation proof (T6).
    PkAgg,
}

impl CircuitName {
    pub fn as_str(&self) -> &'static str {
        match self {
            CircuitName::PkBfv => "pk",
            CircuitName::PkGeneration => "pk_generation",
            CircuitName::SkShareComputation => "sk_share_computation",
            CircuitName::ESmShareComputation => "e_sm_share_computation",
            CircuitName::SkShareEncryption => "sk_share_encryption",
            CircuitName::ESmShareEncryption => "e_sm_share_encryption",
            CircuitName::DecShares => "dec_shares",
            CircuitName::PkAgg => "pk_agg",
        }
    }

    pub fn group(&self) -> &'static str {
        match self {
            CircuitName::PkBfv => "dkg",
            CircuitName::PkGeneration => "threshold",
            CircuitName::SkShareComputation => "dkg",
            CircuitName::ESmShareComputation => "dkg",
            CircuitName::SkShareEncryption => "dkg",
            CircuitName::ESmShareEncryption => "dkg",
            CircuitName::DecShares => "threshold",
            CircuitName::PkAgg => "threshold",
        }
    }

    pub fn dir_path(&self) -> String {
        format!("{}/{}", self.group(), self.as_str())
    }
}

impl fmt::Display for CircuitName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
