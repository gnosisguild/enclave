// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Message, Recipient};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

use super::EnclaveEventData;

/// This is a processed EnclaveEvmEvent
#[derive(Message, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SyncEvmEvent {
    data: EnclaveEventData,
    block: u64,
}

impl SyncEvmEvent {
    pub fn new(data: EnclaveEventData, block: u64) -> Self {
        Self { data, block }
    }
}

/// Dispatched by the Sync actor when initial data is read and the sync process needs to be started
#[derive(Message, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SyncStart {
    #[serde(skip)]
    pub sender: Option<Recipient<SyncEvmEvent>>, // Must be option to allow serde deserialize on
                                                 // EnclaveEvent as Default is required to be
                                                 // implemented
}

impl SyncStart {
    pub fn new(sender: Recipient<SyncEvmEvent>) -> Self {
        Self {
            sender: Some(sender),
        }
    }
}

impl Display for SyncStart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
