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
pub struct NetSyncEventsReceived {
    pub events: Vec<EnclaveEvent<Unsequenced>>,
}

impl NetSyncEventsReceived {
    pub fn new(events: Vec<EnclaveEvent<Unsequenced>>) -> Self {
        Self { events }
    }
}

impl Display for NetSyncEventsReceived {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
