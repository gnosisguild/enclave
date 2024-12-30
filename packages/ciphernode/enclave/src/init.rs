use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input};
use enclave_core::get_tag;
use runtime::init;
use tracing::instrument;

use crate::net;
use crate::net::NetCommands;
use crate::password;
use crate::password::PasswordCommands;

#[instrument(name = "app", skip_all, fields(id = get_tag()))]
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
            init::validate_rpc_url(&url)?;
            url
        }
        None => Input::<String>::new()
            .with_prompt("Enter WebSocket devnet RPC URL")
            .default("wss://ethereum-sepolia-rpc.publicnode.com".to_string())
            .validate_with(init::validate_rpc_url)
            .interact_text()?,
    };

    let eth_address: Option<String> = match eth_address {
        Some(address) => {
            init::validate_eth_address(&address)?;
            Some(address)
        }
        None => {
            if skip_eth {
                None
            } else {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter your Ethereum address (press Enter to skip)")
                    .allow_empty(true)
                    .validate_with(init::validate_eth_address)
                    .interact()
                    .ok()
                    .map(|s| if s.is_empty() { None } else { Some(s) })
                    .flatten()
            }
        }
    };

    let config = init::execute(rpc_url, eth_address).await?;

    password::execute(
        PasswordCommands::Create {
            password,
            overwrite: true,
        },
        &config,
    )
    .await?;

    if generate_net_keypair {
        net::execute(net::NetCommands::GenerateKey, &config).await?;
    } else {
        net::execute(NetCommands::SetKey { net_keypair }, &config).await?;
    }

    println!("Enclave configuration successfully created!");
    println!("You can start your node using `enclave start`");

    Ok(())
}
