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
/// is available. `seq` gives the deterministic ordering.
///
/// The total count of expected proofs is communicated separately via
/// [`ThresholdSharePending`], which is always published before the first
/// `DKGInnerProofReady` for a given E3.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DKGInnerProofReady {
    pub e3_id: E3id,
    pub party_id: u64,
    /// Already-wrapped proof (single-element RecursiveAggregation output).
    pub wrapped_proof: Proof,
    /// Deterministic sequence index for ordered folding.
    pub seq: usize,
}
