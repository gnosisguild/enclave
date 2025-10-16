// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::primitives::Address;
use anyhow::{anyhow, bail, Result};
use e3_config::load_config;
use e3_config::AppConfig;
use e3_config::RPC;
use std::fs;
use tracing::instrument;

// Import a built file:
//   see /target/debug/enclave-xxxxxx/out/contract_deployments.rs
//   also see build.rs
include!(concat!(env!("OUT_DIR"), "/contract_deployments.rs"));

// Get the ContractInfo object
fn get_contract_info(name: &str) -> Result<&ContractInfo> {
    Ok(CONTRACT_DEPLOYMENTS
        .get(name)
        .ok_or(anyhow!("Could not get contract info"))?)
}

pub fn validate_rpc_url(url: &String) -> Result<()> {
    RPC::from_url(url)?;
    Ok(())
}

pub fn validate_eth_address(address: &String) -> Result<()> {
    match Address::parse_checksummed(address, None) {
        Ok(_) => Ok(()),
        Err(e) => bail!("Invalid Ethereum address: {}", e),
    }
}

#[instrument(name = "app", skip_all)]
pub async fn execute(rpc_url: String, eth_address: Option<String>) -> Result<AppConfig> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?
        .join("enclave");

    fs::create_dir_all(&config_dir)?;

    let config_path = config_dir.join("enclave.config.yaml");

    let config_content = format!(
        r#"---
# Enclave Configuration File
{}
chains:
  - name: "devnet"
    rpc_url: "{}"
    contracts:
      enclave:
        address: "{}"
        deploy_block: {}
      ciphernode_registry:
        address: "{}"
        deploy_block: {}
      bonding_registry:
        address: "{}"
        deploy_block: {}
"#,
        eth_address.map_or(String::new(), |addr| format!(
            "# Ethereum Account Configuration\naddress: \"{}\"",
            addr
        )),
        rpc_url,
        get_contract_info("Enclave")?.address,
        get_contract_info("Enclave")?.deploy_block,
        get_contract_info("CiphernodeRegistryOwnable")?.address,
        get_contract_info("CiphernodeRegistryOwnable")?.deploy_block,
        get_contract_info("BondingRegistry")?.address,
        get_contract_info("BondingRegistry")?.deploy_block,
    );

    fs::write(config_path.clone(), config_content)?;

    // Load with default location
    let config = load_config("_default", Some(config_path.display().to_string()), None)?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::validate_eth_address;
    use anyhow::Result;

    #[test]
    fn eth_address_validation() -> Result<()> {
        assert!(
            validate_eth_address(&"0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".to_string()).is_ok()
        );
        assert!(
            validate_eth_address(&"d8dA6BF26964aF9D7eEd9e03E53415D37aA96045".to_string()).is_err()
        );
        assert!(validate_eth_address(&"0x1234567890abcdef".to_string()).is_err());
        assert!(
            validate_eth_address(&"0x0000000000000000000000000000000000000000".to_string()).is_ok()
        );

        Ok(())
    }
}
