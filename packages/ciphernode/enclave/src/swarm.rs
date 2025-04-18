use crate::swarm_up;
use anyhow::*;
use clap::Subcommand;
use config::AppConfig;

#[derive(Subcommand, Debug)]
pub enum SwarmCommands {
    /// Launch all nodes
    Up {
        /// Detached mode: Run nodes in the background
        #[arg(short, long)]
        detatch: bool,
    },

    /// Shutdown all nodes
    Down,
}

pub async fn execute(command: SwarmCommands, config: &AppConfig) -> Result<()> {
    match command {
        SwarmCommands::Up { detatch } => swarm_up::execute(config, detatch).await?,
        SwarmCommands::Down => (),
    };

    Ok(())
}
