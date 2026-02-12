// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{E3id, PkGenerationProofRequest, ThresholdShare};
use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::sync::Arc;

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ThresholdSharePending {
    pub e3_id: E3id,
    /// Full threshold share containing all encrypted shares for all parties
    pub full_share: Arc<ThresholdShare>,
    /// The proof request data for the zk actor
    pub proof_request: PkGenerationProofRequest,
}

impl Display for ThresholdSharePending {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
