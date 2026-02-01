// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use super::EvmSyncEventsReceived;
use crate::{EvmEventConfig, EvmEventConfigChain};
use actix::{Message, Recipient};
use anyhow::Context;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// Dispatched by the Sync actor when initial data is read and the sync process needs to be started
#[derive(Message, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SyncStart {
    /// The initial information for reading historical events from chains. This is generated from
    /// from persisted information
    pub evm_config: EvmEventConfig,

    #[serde(skip)]
    /// We include the sender here so that the evm can communicate directly with the sync actor
    pub sender: Option<Recipient<EvmSyncEventsReceived>>, // Must be Option to allow serde deserialize on
                                                          // EnclaveEvent as Default is required to be
                                                          // implemented this is fine as this event is never
                                                          // shared
}

impl SyncStart {
    pub fn new(
        sender: impl Into<Recipient<EvmSyncEventsReceived>>,
        evm_config: EvmEventConfig,
    ) -> Self {
        Self {
            sender: Some(sender.into()),
            evm_config,
        }
    }

    pub fn get_evm_config(&self, chain_id: u64) -> Result<EvmEventConfigChain> {
        Ok(self
            .evm_config
            .get(&chain_id)
            .context("No config found for chain")?
            .clone())
    }
}

impl Display for SyncStart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
