// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input};
use e3_entrypoint::config::setup;
use e3_utils::eth_address_from_private_key;
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
        None => Input::<String>::new()
            .with_prompt("Enter WebSocket devnet RPC URL")
            .default("wss://ethereum-sepolia-rpc.publicnode.com".to_string())
            .validate_with(setup::validate_rpc_url)
            .interact_text()?,
    };

    let private_key = ask_for_private_key(private_key)?;
    let address = eth_address_from_private_key(&private_key)?;
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
    let config = setup::execute(rpc_url, &config_dir, &address)?;

    password_set::execute(&config, Some(pw)).await?;

    e3_entrypoint::wallet::set::execute(&config, private_key).await?;

    println!("Enclave configuration successfully created!");
    println!("Configuration has been written to {:?}", config_dir);
    println!(
        "Run future commands from within this directory tree, or pass --config {:?}",
        config_dir
    );

    Ok(())
}
