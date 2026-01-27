// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::collections::HashMap;

use alloy::primitives::Address;
use anyhow::Result;
use e3_config::chain_config::ChainConfig;
use e3_events::EvmEventConfig;
type ChainId = u64;
type DeployBlock = Option<u64>;

pub struct SyncBuilder {
    config: EvmEventConfig,
}

impl SyncBuilder {
    pub fn with_chain(&mut self, chain_id: u64, chain: ChainConfig) -> Result<()> {
        self.config.insert(chain_id, chain.try_into()?);
        Ok(())
    }
}
