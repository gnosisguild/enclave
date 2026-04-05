// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::E3id;
use actix::Message;
use alloy::primitives::Address;
use e3_utils::ArcBytes;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// Broadcast via gossip: a committee member's vote on an accusation.
///
/// Each committee member independently checks whether the accused's proof
/// failed verification from their perspective, and broadcasts this vote.
#[derive(Message, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AccusationVote {
    pub e3_id: E3id,
    /// keccak256 of the `ProofFailureAccusation` this vote responds to.
    pub accusation_id: [u8; 32],
    /// Ethereum address of the voter.
    pub voter: Address,
    /// `true` if this node also saw the proof fail verification.
    pub agrees: bool,
    /// keccak256 hash of the data as this node received it — for equivocation detection.
    pub data_hash: [u8; 32],
    /// ECDSA signature of the voter over the vote fields.
    pub signature: ArcBytes,
}

impl Display for AccusationVote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AccusationVote {{ e3_id: {}, voter: {}, agrees: {} }}",
            self.e3_id, self.voter, self.agrees
        )
    }
}
