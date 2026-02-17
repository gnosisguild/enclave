// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::path::PathBuf;
use std::str::FromStr;

use alloy::primitives::Address;
use alloy::signers::local::PrivateKeySigner;
use anyhow::Context;
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use e3_entrypoint::config_set;
use tracing::instrument;
use zeroize::Zeroizing;

use crate::net::{NetCommands, NetKeypairCommands};
use crate::password_set::ask_for_password;
use crate::wallet_set::ask_for_private_key;
use crate::{net, password_set};

fn eth_address_from_private_key(private_key: &str) -> Result<Address> {
    let signer = PrivateKeySigner::from_str(private_key).context("invalid private key")?;
    Ok(signer.address())
}

#[instrument(name = "app", skip_all)]
pub async fn execute(
    rpc_url: Option<String>,
    password: Option<Zeroizing<String>>,
    private_key: Option<Zeroizing<String>>,
    net_keypair: Option<String>,
    generate_net_keypair: bool,
) -> Result<()> {
    let pw = ask_for_password(password)?;
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
    let config = config_set::execute(rpc_url, &config_dir, &address)?;

    password_set::execute(&config, Some(pw)).await?;

    e3_entrypoint::wallet::set::execute(&config, private_key).await?;

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
