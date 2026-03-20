// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{E3id, Proof};
use actix::Message;
use derivative::Derivative;
use e3_utils::ArcBytes;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Derivative, Message, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
#[rtype(result = "()")]
pub struct PlaintextAggregated {
    pub e3_id: E3id,
    pub decrypted_output: Vec<ArcBytes>,
    /// C7 proofs: one proof of correct aggregation per ciphertext index.
    #[serde(default)]
    pub aggregation_proofs: Vec<Proof>,
    /// Cross-node folded C6 proof: all honest nodes' threshold share decryption proofs folded into one.
    #[serde(default)]
    pub c6_aggregated_proof: Option<Proof>,
}

impl Display for PlaintextAggregated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
