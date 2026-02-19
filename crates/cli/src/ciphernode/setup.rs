// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::primitives::Address;
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input};
use e3_config::AppConfig;
use e3_entrypoint::config::setup;
use e3_utils::{colorize, eth_address_from_private_key, Color};
use std::path::PathBuf;
use tracing::instrument;
use zeroize::Zeroizing;

use crate::password_set;
use crate::password_set::ask_for_password;
use crate::wallet_set::ask_for_private_key;

#[instrument(name = "app", skip_all)]
pub async fn execute(
    rpc_url: Option<String>,
    password: Option<Zeroizing<String>>,
    private_key: Option<Zeroizing<String>>,
) -> Result<()> {
    let pw = ask_for_password(password)?;
    let rpc_url = match rpc_url {
        Some(url) => {
            setup::validate_rpc_url(&url)?;
            url
        }
        None => Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter WebSocket devnet RPC URL")
            .default("wss://ethereum-sepolia-rpc.publicnode.com".to_string())
            .validate_with(setup::validate_rpc_url)
            .interact_text()?,
    };

    let private_key = ask_for_private_key(private_key)?;
    let default_config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join("enclave");

    let config_dir: PathBuf = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter config directory")
        .default(default_config_dir.display().to_string())
        .validate_with(|input: &String| -> Result<(), &str> {
            let path = PathBuf::from(input);
            if input.is_empty() {
                Err("Path cannot be empty")
            } else if path.is_file() {
                Err("Path is a file, not a directory")
            } else {
                Ok(())
            }
        })
        .interact_text()?
        .into();

    // Execute
    let config = setup::execute(&rpc_url, &config_dir)?;

    e3_entrypoint::password::set::preflight(&config).await?;
    e3_entrypoint::password::set::execute(&config, pw).await?;

    let (address, peer_id) = e3_entrypoint::wallet::set::execute(&config, private_key).await?;
    print_info(&config, address, &peer_id.to_string(), &rpc_url)?;
    Ok(())
}

fn print_info(config: &AppConfig, address: Address, peer_id: &str, rpc_url: &str) -> Result<()> {
    let abs_config = config.config_file().canonicalize()?;

    println!("\nEnclave configuration successfully created!");
    println!(
        "Editable configuration has been written to:\n\n {}",
        colorize(abs_config.to_string_lossy(), Color::Yellow)
    );
    println!("");
    println!("Data written:");
    println!(" address: {}", colorize(address, Color::Cyan));
    println!(" peer_id: {}", colorize(peer_id, Color::Cyan));
    println!(" rpc_url: {}", colorize(rpc_url, Color::Cyan));
    println!("");
    if config.using_custom_config() {
        println!(
            "Run future commands from within this directory tree, or pass\n {}\n",
            colorize(
                format!("--config {}", abs_config.to_string_lossy()),
                Color::Yellow
            )
        );
    }
    println!(
        "You can start your node using:\n `{}`\n",
        colorize("enclave start", Color::Yellow)
    );
    Ok(())
}
