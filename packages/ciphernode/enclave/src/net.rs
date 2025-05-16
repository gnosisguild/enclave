use anyhow::*;
use clap::Subcommand;
use config::AppConfig;

use crate::{net_generate, net_purge, net_set};

#[derive(Subcommand, Debug)]
pub enum NetCommands {
    /// Generate new net keypair
    Keypair {
        #[command(subcommand)]
        command: NetKeypairCommands,
    },

    /// Purge peer ID
    #[command(name = "peer-id")]
    PeerId {
        #[command(subcommand)]
        command: NetPeerIdCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum NetKeypairCommands {
    /// Generate new net keypair
    Generate,

    /// Set net private key
    Set {
        #[arg(long = "net-keypair")]
        net_keypair: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum NetPeerIdCommands {
    /// Purge peer ID
    Purge,
}

pub async fn execute(command: NetCommands, config: &AppConfig) -> Result<()> {
    match command {
        NetCommands::Keypair { command } => match command {
            NetKeypairCommands::Generate => net_generate::execute(&config).await?,
            NetKeypairCommands::Set { net_keypair } => net_set::execute(&config, net_keypair).await?,
        },
        NetCommands::PeerId { command } => match command {
            NetPeerIdCommands::Purge => net_purge::execute(&config).await?,
        },
    };

    Ok(())
}
