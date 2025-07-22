// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_config::AppConfig;

pub fn extract_env_vars_vite(config: &AppConfig, chain: &str) -> String {
    let mut env_vars = Vec::new();

    // Extract from first enabled chain (or first chain if none specified)
    if let Some(chain) = config.chains().iter().find(|c| c.name == chain.to_string()) {
        let enclave_addr = &chain.contracts.enclave;
        let registry_addr = &chain.contracts.ciphernode_registry;
        let filter_addr = &chain.contracts.filter_registry;
        env_vars.push(format!("VITE_ENCLAVE_ADDRESS={}", enclave_addr.address()));
        env_vars.push(format!("VITE_REGISTRY_ADDRESS={}", registry_addr.address()));
        env_vars.push(format!("VITE_RPC_URL={}", chain.rpc_url));
        env_vars.push(format!(
            "VITE_FILTER_REGISTRY_ADDRESS={}",
            filter_addr.address()
        ));
        if let Some(e3_program) = &chain.contracts.e3_program {
            env_vars.push(format!("VITE_E3_PROGRAM_ADDRESS={}", e3_program.address()));
        }
    }

    env_vars.join(" ")
}

pub fn extract_env_vars(config: &AppConfig, chain: &str) -> String {
    let mut env_vars = Vec::new();

    // Extract from first enabled chain (or first chain if none specified)
    if let Some(chain) = config.chains().iter().find(|c| c.name == chain.to_string()) {
        let enclave_addr = &chain.contracts.enclave;
        let registry_addr = &chain.contracts.ciphernode_registry;
        let filter_addr = &chain.contracts.filter_registry;
        env_vars.push(format!("ENCLAVE_ADDRESS={}", enclave_addr.address()));
        env_vars.push(format!("RPC_URL={}", chain.rpc_url));
        env_vars.push(format!("REGISTRY_ADDRESS={}", registry_addr.address()));
        env_vars.push(format!("FILTER_REGISTRY_ADDRESS={}", filter_addr.address()));
        if let Some(e3_program) = &chain.contracts.e3_program {
            env_vars.push(format!("E3_PROGRAM_ADDRESS={}", e3_program.address()));
        }
    }

    env_vars.join(" ")
}
pub async fn execute(config: &AppConfig, chain: &str, as_vite: bool) -> Result<()> {
    if as_vite {
        println!("{}", extract_env_vars_vite(config, chain));
    } else {
        println!("{}", extract_env_vars(config, chain));
    }
    Ok(())
}
