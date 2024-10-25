use crate::commands::{aggregator, password, start, Commands};
use anyhow::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "mycli")]
#[command(about = "A CLI application", long_about = None)]
pub struct Cli {
    /// Path to config file
    #[arg(long, global = true)]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn execute(self) -> Result<()> {
        let config_path = self.config.as_deref();

        match self.command {
            Commands::Start { address } => {
                start::execute(config_path, &address).await?
            }
            Commands::Password { command } => {
                password::execute(command, config_path).await?
            }
            Commands::Aggregator { command } => {
                aggregator::execute(command, config_path).await?
            }
        }

        Ok(())
    }
}
