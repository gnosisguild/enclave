// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::E3id;
use actix::Message;
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    sync::Arc,
};

/// Type Representing Pvw encrypted bytes
pub type PvwBytes = Vec<u8>;

/// PVW encrypted shares list for a party in the DKG
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct ThresholdShare {
    /// The publishers party_id
    pub party_id: u64,
    /// The publishers public key share
    #[derivative(Debug(format_with = "crate::hexf"))]
    pub pk_share: Arc<Vec<u8>>,
    /// PVW encrypted sk_sss list with index determining party_id
    #[derivative(Debug(format_with = "crate::hexf_bytes_slice"))]
    pub sk_sss: Vec<PvwBytes>,
    /// PVW encrypted esi_sss list with index determining party_id
    #[derivative(Debug(format_with = "crate::hexf_3d_bytes"))]
    pub esi_sss: Vec<Vec<PvwBytes>>,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ThresholdShareCreated {
    pub e3_id: E3id,
    pub share: Arc<ThresholdShare>,
}

impl Display for ThresholdShareCreated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e3_id: {}", self.e3_id)
    }
}
