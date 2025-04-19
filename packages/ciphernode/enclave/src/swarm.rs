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

        /// Exclude nodes by name
        #[arg(short, long, value_delimiter = ',')]
        exclude: Vec<String>,
    },

    /// Shutdown all nodes
    Down,

    Daemon {
        /// Detached mode: Run nodes in the background
        #[arg(short, long)]
        detatch: bool,

        /// Exclude nodes by name
        #[arg(short, long, value_delimiter = ',')]
        exclude: Vec<String>,
    },
}

pub async fn execute(
    command: SwarmCommands,
    config: &AppConfig,
    verbose: u8,
    config_string: Option<String>,
) -> Result<()> {
    match command {
        SwarmCommands::Up { detatch, exclude } => {
            swarm_up::execute(config, detatch, exclude, verbose, config_string).await?
        }
        SwarmCommands::Down => (),
        SwarmCommands::Daemon { detatch, exclude } => {
            swarm_up::execute(config, detatch, exclude, verbose, config_string).await?
        }
    };

    Ok(())
}
