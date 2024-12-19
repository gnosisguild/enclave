mod generate;
mod purge;
mod set;
use anyhow::*;
use clap::Subcommand;
use config::AppConfig;

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
        NetCommands::PurgeId => purge::execute(&config).await?,
        NetCommands::GenerateKey => generate::execute(&config).await?,
        NetCommands::SetKey { net_keypair } => set::execute(&config, net_keypair).await?,
    };

    Ok(())
}
