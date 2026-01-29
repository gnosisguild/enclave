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
    /// TrBFV public key share proof (T1).
    PkTrbfv,
    /// Encrypted shares proof (T2/T3).
    EncShares,
    /// Decryption share proof (T4/T5).
    DecShares,
    /// Public key aggregation proof (T6).
    PkAgg,
}

impl CircuitName {
    /// Get the file name for this circuit.
    pub fn as_str(&self) -> &'static str {
        match self {
            CircuitName::PkBfv => "pk_bfv",
            CircuitName::PkTrbfv => "pk_trbfv",
            CircuitName::EncShares => "enc_shares",
            CircuitName::DecShares => "dec_shares",
            CircuitName::PkAgg => "pk_agg",
        }
    }
}

impl fmt::Display for CircuitName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
