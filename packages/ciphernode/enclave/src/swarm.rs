use crate::{
    swarm_daemon, swarm_down, swarm_ps, swarm_restart, swarm_start, swarm_status, swarm_stop,
    swarm_up,
};
use anyhow::*;
use clap::Subcommand;
use config::AppConfig;

#[derive(Subcommand, Debug)]
pub enum NodeCommands {
    /// Launch all nodes
    Up {
        /// Detached mode: Run nodes in the background
        #[arg(short, long)]
        detach: bool,

        /// Exclude nodes by name
        #[arg(short, long, value_delimiter = ',')]
        exclude: Vec<String>,
    },

    /// Shutdown all nodes
    Down,

    Daemon {
        /// Exclude nodes by name
        #[arg(short, long, value_delimiter = ',')]
        exclude: Vec<String>,
    },

    /// List all process statuses
    Ps,

    /// Start an individual node in the nodes set
    Start {
        /// The id of the node
        #[arg(index = 1)]
        id: String,
    },

    /// Stop the individual node in the nodes set
    Stop {
        /// The id of the node
        #[arg(index = 1)]
        id: String,
    },

    /// Print the status of the individual node in the nodes set
    Status {
        /// The id of the node
        #[arg(index = 1)]
        id: String,
    },

    /// Stop the individual node in the nodes set
    Restart {
        /// The id of the node
        #[arg(index = 1)]
        id: String,
    },
}

pub async fn execute(
    command: NodeCommands,
    config: &AppConfig,
    verbose: u8,
    config_string: Option<String>,
) -> Result<()> {
    match command {
        NodeCommands::Up { detach, exclude } => {
            swarm_up::execute(config, detach, exclude, verbose, config_string).await?
        }
        NodeCommands::Down => swarm_down::execute().await?,
        NodeCommands::Ps => swarm_ps::execute().await?,
        NodeCommands::Daemon { exclude } => {
            swarm_daemon::execute(config, exclude, verbose, config_string).await?
        }
        NodeCommands::Start { id } => swarm_start::execute(&id).await?,
        NodeCommands::Status { id } => swarm_status::execute(&id).await?,
        NodeCommands::Stop { id } => swarm_stop::execute(&id).await?,
        NodeCommands::Restart { id } => swarm_restart::execute(&id).await?,
    };

    Ok(())
}
