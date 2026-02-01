// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

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
