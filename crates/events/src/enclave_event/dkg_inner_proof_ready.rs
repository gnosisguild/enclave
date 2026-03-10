// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Event emitted by [`ProofRequestActor`] for each inner DKG proof as it completes.
//!
//! [`NodeProofAggregator`] consumes these events and folds them incrementally
//! in strict `seq` order into a single aggregated proof.

use crate::{E3id, Proof};
use serde::{Deserialize, Serialize};

/// A single wrapped inner proof ready for incremental aggregation.
///
/// Emitted for every inner circuit (C0-C4) as soon as its wrapped proof
/// is available. `seq` gives the deterministic ordering; `total_expected`
/// lets the aggregator know when the stream is complete.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DKGInnerProofReady {
    pub e3_id: E3id,
    pub party_id: u64,
    pub node: String,
    /// Already-wrapped proof (single-element RecursiveAggregation output).
    pub wrapped_proof: Proof,
    /// Deterministic sequence index for ordered folding.
    pub seq: usize,
    /// Total number of inner proofs expected for this E3 node.
    pub total_expected: usize,
}
