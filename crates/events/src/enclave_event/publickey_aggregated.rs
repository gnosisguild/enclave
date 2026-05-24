// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{E3id, OrderedSet, Proof};
use actix::Message;
use alloy::primitives::Address;
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
    /// Committee addresses in ascending `party_id` (score) order for hash binding.
    #[serde(default)]
    pub committee_addresses: Vec<Address>,
    /// Hash-based aggregated PK commitment (last public signal of the C5 proof).
    /// Passed as `pkCommitment` to `publishCommittee`.
    pub pk_commitment: [u8; 32],
    /// EVM DKG recursive proof (`CircuitName::DkgAggregator`) carrying node folds + C5
    /// for on-chain verification. `None` when proof aggregation is disabled.
    #[serde(default)]
    pub dkg_aggregator_proof: Option<Proof>,
    /// ABI-encoded `(Attestation[], PartySlotBinding[])` for on-chain fold attestation verify.
    /// Required when `dkg_aggregator_proof` is present; `None` otherwise.
    #[serde(default)]
    pub dkg_attestation_bundle: Option<ArcBytes>,
}

impl Display for PublicKeyAggregated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
