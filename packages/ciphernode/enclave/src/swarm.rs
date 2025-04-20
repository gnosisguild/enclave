use crate::{swarm_down, swarm_start, swarm_stop, swarm_up};
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

    Start {
        /// The id of the node you wish to start
        #[arg(index = 1)]
        id: String,
    },

    Stop {
        /// The id of the node you wish to start
        #[arg(index = 1)]
        id: String,
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
        SwarmCommands::Down => swarm_down::execute().await?,
        SwarmCommands::Daemon { detatch, exclude } => {
            swarm_up::execute(config, detatch, exclude, verbose, config_string).await?
        }
        SwarmCommands::Start { id } => swarm_start::execute(&id).await?,
        SwarmCommands::Stop { id } => swarm_stop::execute(&id).await?,
    };

    Ok(())
}
