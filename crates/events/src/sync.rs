// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::collections::{BTreeMap, HashSet};

use crate::EvmEvent;
use actix::Message;
use serde::{Deserialize, Serialize};
type Chainid = u64;
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub enum SyncEvmEvent {
    /// Signal that this reader has completed historical sync
    HistoricalSyncComplete(ChainId),
    /// An actual event from the blockchain
    Event(EvmEvent),
}

impl From<EvmEvent> for SyncEvmEvent {
    fn from(event: EvmEvent) -> SyncEvmEvent {
        SyncEvmEvent::Event(event)
    }
}

type ChainId = u64;
type DeployBlock = u64;

/// Configuration value object for starting the evm reader for a specific chain
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvmEventConfigChain {
    deploy_block: DeployBlock,
}

impl EvmEventConfigChain {
    pub fn new(deploy_block: DeployBlock) -> Self {
        Self { deploy_block }
    }
    pub fn deploy_block(&self) -> u64 {
        self.deploy_block
    }
}

/// Configuration value object for starting the evm reader for all chains
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvmEventConfig {
    config: BTreeMap<ChainId, EvmEventConfigChain>, // Need BTreeMap because of Hash
}

impl EvmEventConfig {
    pub fn new() -> Self {
        Self {
            config: BTreeMap::new(),
        }
    }
    pub fn get(&self, chain_id: &ChainId) -> Option<&EvmEventConfigChain> {
        self.config.get(&chain_id)
    }

    pub fn insert(&mut self, key: ChainId, value: EvmEventConfigChain) {
        self.config.insert(key, value);
    }

    pub fn chains(&self) -> HashSet<u64> {
        self.config.keys().cloned().collect()
    }
}
