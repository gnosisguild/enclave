// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::path::PathBuf;

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use e3_entrypoint::config_set;
use tracing::instrument;

use crate::net;
use crate::net::{NetCommands, NetKeypairCommands};
use crate::password;
use crate::password::PasswordCommands;

#[instrument(name = "app", skip_all)]
pub async fn execute(
    rpc_url: Option<String>,
    eth_address: Option<String>,
    password: Option<String>,
    skip_eth: bool,
    net_keypair: Option<String>,
    generate_net_keypair: bool,
) -> Result<()> {
    let rpc_url = match rpc_url {
        Some(url) => {
            config_set::validate_rpc_url(&url)?;
            url
        }
        None => Input::<String>::new()
            .with_prompt("Enter WebSocket devnet RPC URL")
            .default("wss://ethereum-sepolia-rpc.publicnode.com".to_string())
            .validate_with(config_set::validate_rpc_url)
            .interact_text()?,
    };

    let eth_address: Option<String> = match eth_address {
        Some(address) => {
            config_set::validate_eth_address(&address)?;
            Some(address)
        }
        None => {
            if skip_eth {
                None
            } else {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter your Ethereum address (press Enter to skip)")
                    .allow_empty(true)
                    .validate_with(config_set::validate_eth_address)
                    .interact()
                    .ok()
                    .map(|s| if s.is_empty() { None } else { Some(s) })
                    .flatten()
            }
        }
    };

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

    let net_keypair_command = if generate_net_keypair {
        NetKeypairCommands::Generate
    } else if net_keypair.is_none() {
        let should_generate = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("No net keypair specified. Would you like to generate one automatically?")
            .default(true)
            .interact()?;
        if should_generate {
            NetKeypairCommands::Generate
        } else {
            NetKeypairCommands::Set { net_keypair }
        }
    } else {
        NetKeypairCommands::Set { net_keypair }
    };

    // Execute

    let config = config_set::execute(rpc_url, eth_address, &config_dir)?;

    for i in 0..3 {
        if password::execute(
            PasswordCommands::Set {
                password: password.clone(),
            },
            &config,
        )
        .await
        .is_ok()
        {
            break;
        }
        if i == 2 {
            return Err(anyhow::anyhow!("Failed after 3 attempts"));
        }
    }

    net::execute(
        NetCommands::Keypair {
            command: net_keypair_command,
        },
        &config,
    )
    .await?;

    println!("Enclave configuration successfully created!");

    Ok(())
}
