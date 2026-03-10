// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Event published by [`NodeProofAggregator`] when all inner proofs for a
//! DKG node have been incrementally folded into a single aggregated proof.
//!
//! [`PublicKeyAggregator`] collects these from all honest nodes for the
//! cross-node aggregation phase.

use crate::{E3id, Proof};
use serde::{Deserialize, Serialize};

/// NodeProofAggregator -> PublicKeyAggregator: fully aggregated DKG node proof.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DKGRecursiveAggregationComplete {
    pub e3_id: E3id,
    pub party_id: u64,
    pub node: String,
    pub aggregated_proof: Proof,
}
