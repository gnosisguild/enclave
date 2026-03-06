// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{AccusationVote, E3id, ProofType};
use actix::Message;
use alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// The outcome of an accusation quorum vote.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccusationOutcome {
    /// >= M nodes agree the proof is bad → slash the accused.
    AccusedFaulted,
    /// Only the accuser says bad, same data_hash as others → accuser lied.
    AccuserLied,
    /// data_hashes differ between voters → accused sent different data to different nodes.
    Equivocation,
    /// Vote timeout expired or not enough votes → proceed with E3 timeout.
    Inconclusive,
}

impl Display for AccusationOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccusationOutcome::AccusedFaulted => write!(f, "AccusedFaulted"),
            AccusationOutcome::AccuserLied => write!(f, "AccuserLied"),
            AccusationOutcome::Equivocation => write!(f, "Equivocation"),
            AccusationOutcome::Inconclusive => write!(f, "Inconclusive"),
        }
    }
}

/// Emitted locally when the accusation quorum protocol reaches a decision.
///
/// Consumed by aggregator actors to exclude faulted nodes and by the on-chain
/// submission logic to submit `E3FaultEvidence`.
#[derive(Message, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AccusationQuorumReached {
    pub e3_id: E3id,
    /// Address of the node that originally accused.
    pub accuser: Address,
    /// Address of the accused node.
    pub accused: Address,
    /// Which proof type was disputed.
    pub proof_type: ProofType,
    /// Votes from nodes that agreed the proof is bad.
    pub votes_for: Vec<AccusationVote>,
    /// Votes from nodes that said the proof is fine.
    pub votes_against: Vec<AccusationVote>,
    /// The quorum decision.
    pub outcome: AccusationOutcome,
}

impl Display for AccusationQuorumReached {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AccusationQuorumReached {{ e3_id: {}, accused: {}, proof_type: {:?}, outcome: {} }}",
            self.e3_id, self.accused, self.proof_type, self.outcome
        )
    }
}
