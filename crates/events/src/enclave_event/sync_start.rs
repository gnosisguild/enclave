// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use super::EnclaveEventData;
use crate::SyncEvmEvent;
use actix::{Message, Recipient};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Display},
};

/// This is a processed EvmEvent specifically typed for the Sync actor
#[derive(Message, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct EvmEvent {
    data: EnclaveEventData,
    block: u64,
    chain_id: u64,
    ts: u128,
}

impl EvmEvent {
    pub fn new(data: EnclaveEventData, block: u64, ts: u128, chain_id: u64) -> Self {
        Self {
            data,
            block,
            ts,
            chain_id,
        }
    }

    pub fn split(self) -> (EnclaveEventData, u128, u64) {
        (self.data, self.ts, self.block)
    }
}

/// Dispatched by the Sync actor when initial data is read and the sync process needs to be started
#[derive(Message, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SyncStart {
    /// The start block information for chains
    pub evm_init_info: Vec<(u64, Option<u64>)>, // HashMap cannot derive Hash
    #[serde(skip)]
    pub sender: Option<Recipient<SyncEvmEvent>>, // Must be Option to allow serde deserialize on
                                                 // EnclaveEvent as Default is required to be
                                                 // implemented
}

impl SyncStart {
    pub fn new(
        sender: impl Into<Recipient<SyncEvmEvent>>,
        evm_init_info: HashMap<u64, Option<u64>>,
    ) -> Self {
        Self {
            sender: Some(sender.into()),
            evm_init_info: evm_init_info.into_iter().collect(),
        }
    }

    pub fn get_evm_init_for(&self, chain_id: u64) -> Option<u64> {
        self.evm_init_info
            .iter()
            .find_map(|(ch_id, value)| {
                if ch_id == &chain_id {
                    Some(value.clone())
                } else {
                    None
                }
            })
            .unwrap_or(None)
    }
}

impl Display for SyncStart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
