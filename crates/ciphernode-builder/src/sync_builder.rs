use std::collections::HashMap;

use alloy::primitives::Address;
use e3_config::chain_config::ChainConfig;

type ChainId = u64;
type DeployBlock = u64;

pub struct SyncBuilder {
    config: HashMap<ChainConfig, HashMap<Address, DeployBlock>>,
}

impl SyncBuilder {
    pub fn with_chain(&mut self, chain: &ChainConfig) {
        let contracts = chain.contracts.contracts();
        let mut map = HashMap::new();
        for contract in contracts {
            let key = contract.address();
            let value = contract.deploy_block();
            map.insert(key, value);
        }
        self.config.insert(chain, map);
    }
}
