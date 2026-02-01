// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

use super::{EnclaveEvent, Unsequenced};

#[derive(Message, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct EvmSyncEventsReceived {
    pub events: Vec<EnclaveEvent<Unsequenced>>,
    pub chain_id: u64,
}

impl EvmSyncEventsReceived {
    pub fn new(events: Vec<EnclaveEvent<Unsequenced>>, chain_id: u64) -> Self {
        Self { events, chain_id }
    }
}

impl Display for EvmSyncEventsReceived {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
