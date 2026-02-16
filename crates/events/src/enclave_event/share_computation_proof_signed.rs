// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{E3id, ProofType, SignedProofPayload};
use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ShareComputationProofSigned {
    pub e3_id: E3id,
    pub party_id: u64,
    pub proof_type: ProofType,
    pub signed_proof: SignedProofPayload,
}

impl Display for ShareComputationProofSigned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
