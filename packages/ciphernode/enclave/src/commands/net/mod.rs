
mod purge;
use anyhow::*;
use clap::Subcommand;
use config::AppConfig;

#[derive(Subcommand, Debug)]
pub enum NetCommands {
    /// Purge the current peer ID from the database. 
    PurgeId
}

pub async fn execute(command: NetCommands, config: AppConfig) -> Result<()> {
    match command {
        NetCommands::PurgeId => purge::execute(&config).await?,
    };

    Ok(())
}
