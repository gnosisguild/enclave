use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input};
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

    let config = config_set::execute(rpc_url, eth_address).await?;

    password::execute(PasswordCommands::Set { password }, &config).await?;

    if generate_net_keypair {
        net::execute(
            NetCommands::Keypair {
                command: NetKeypairCommands::Generate,
            },
            &config,
        )
        .await?;
    } else {
        net::execute(
            NetCommands::Keypair {
                command: NetKeypairCommands::Set { net_keypair },
            },
            &config,
        )
        .await?;
    }

    println!("Enclave configuration successfully created!");

    Ok(())
}
