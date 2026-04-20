// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{E3id, OrderedSet, Proof};
use actix::Message;
use derivative::Derivative;
use e3_utils::ArcBytes;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Derivative, Message, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
#[rtype(result = "()")]
pub struct PublicKeyAggregated {
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub pubkey: ArcBytes, // TODO: ArcBytes ?
    pub e3_id: E3id,
    pub nodes: OrderedSet<String>,
    /// Safe-based aggregated PK commitment (last public signal of the C5 proof).
    /// Always present; forwarded to `publishCommittee(... pkCommitment ...)`.
    #[serde(default)]
    pub pk_commitment: [u8; 32],
    /// EVM DKG recursive proof (`CircuitName::DkgAggregator`) carrying node folds + C5
    /// for on-chain verification. `None` when proof aggregation is disabled.
    #[serde(default)]
    pub dkg_aggregator_proof: Option<Proof>,
}

impl Display for PublicKeyAggregated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
