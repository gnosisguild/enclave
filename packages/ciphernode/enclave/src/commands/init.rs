use crate::commands::password::{self, PasswordCommands};
use alloy::transports::http::reqwest::Url;
use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use config::load_config;
use config::RPC;
use dialoguer::{theme::ColorfulTheme, Input};
use enclave_core::get_tag;
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

#[instrument(name = "app", skip_all, fields(id = get_tag()))]
pub async fn execute() -> Result<()> {
    let rpc_url = Input::<String>::new()
        .with_prompt("Enter WebSocket devnet RPC URL")
        .default("wss://ethereum-sepolia-rpc.publicnode.com".to_string())
        .validate_with(|input: &String| -> Result<()> {
            RPC::from_url(input)?;
            Ok(())
        })
        .interact_text()?;

    let eth_address: Option<String> = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter your Ethereum address (press Enter to skip)")
        .allow_empty(true)
        .validate_with(|input: &String| -> Result<()> {
            if input.is_empty() {
                return Ok(());
            }
            if !input.starts_with("0x") {
                bail!("Address must start with '0x'")
            }
            if input.len() != 42 {
                bail!("Address must be 42 characters long (including '0x')")
            }
            for c in input[2..].chars() {
                if !c.is_ascii_hexdigit() {
                    bail!("Address must contain only hexadecimal characters")
                }
            }
            Ok(())
        })
        .interact()
        .ok()
        .map(|s| if s.is_empty() { None } else { Some(s) })
        .flatten();

    let config_dir = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?
        .join(".config")
        .join("enclave");
    fs::create_dir_all(&config_dir)?;

    let config_path = config_dir.join("config.yaml");

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
      filter_registry:
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
        get_contract_info("NaiveRegistryFilter")?.address,
        get_contract_info("NaiveRegistryFilter")?.deploy_block,
    );

    fs::write(config_path.clone(), config_content)?;

    // Load with default location
    let config = load_config(Some(&config_path.display().to_string()))?;

    password::execute(PasswordCommands::Create { password: None }, config).await?;

    println!("Enclave configuration successfully created!");
    println!("You can start your node using `enclave start`");

    Ok(())
}
