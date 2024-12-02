
mod purge;
use anyhow::*;
use clap::Subcommand;
use config::AppConfig;

#[derive(Subcommand, Debug)]
pub enum P2pCommands {
    /// Purge the current peer id from the database. 
    Purge
}

pub async fn execute(command: P2pCommands, config: AppConfig) -> Result<()> {
    match command {
        P2pCommands::Purge => purge::execute(&config).await?,
    };

    Ok(())
}
