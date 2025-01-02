use anyhow::*;
use clap::Subcommand;
use config::AppConfig;

use crate::{net_generate, net_purge, net_set};

#[derive(Subcommand, Debug)]
pub enum NetCommands {
    /// Purge the current peer ID from the database.
    PurgeId,

    /// Generate a new network keypair
    GenerateKey,

    /// Set the network private key
    SetKey {
        #[arg(long = "net-keypair")]
        net_keypair: Option<String>,
    },
}

pub async fn execute(command: NetCommands, config: &AppConfig) -> Result<()> {
    match command {
        NetCommands::PurgeId => net_purge::execute(&config).await?,
        NetCommands::GenerateKey => net_generate::execute(&config).await?,
        NetCommands::SetKey { net_keypair } => net_set::execute(&config, net_keypair).await?,
    };

    Ok(())
}
