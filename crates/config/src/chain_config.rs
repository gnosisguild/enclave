use crate::{
    contract::ContractAddresses,
    rpc::{RpcAuth, RPC},
};
use anyhow::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ChainConfig {
    pub enabled: Option<bool>,
    pub name: String,
    pub rpc_url: String, // We may need multiple per chain for redundancy at a later point
    #[serde(default)]
    pub rpc_auth: RpcAuth,
    pub contracts: ContractAddresses,
}

impl ChainConfig {
    pub fn rpc_url(&self) -> Result<RPC> {
        Ok(RPC::from_url(&self.rpc_url)
            .map_err(|e| anyhow!("Failed to parse RPC URL for chain {}: {}", self.name, e))?)
    }
}
