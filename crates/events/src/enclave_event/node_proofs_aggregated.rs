// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Event for node-side proof aggregation completion.
//!
//! `NodeProofsAggregated` is published by [`ProofRequestActor`] after
//! all C0-C4 inner proofs have been generated and aggregated into a
//! single wrapper+fold proof. [`PublicKeyAggregator`] collects these
//! from H honest nodes for the Phase 1 aggregation.

use crate::{E3id, Proof};
use serde::{Deserialize, Serialize};

/// ProofRequestActor -> PublicKeyAggregator: node's aggregated C0-C4 proof.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeProofsAggregated {
    pub e3_id: E3id,
    /// The party index of this node.
    pub party_id: u64,
    /// The node identifier.
    pub node: String,
    /// The single aggregated proof covering C0-C4 circuits.
    pub aggregated_proof: Proof,
}
