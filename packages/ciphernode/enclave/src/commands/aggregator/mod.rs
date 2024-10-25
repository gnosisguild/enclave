mod start;
use anyhow::*;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum AggregatorCommands {
    /// Start the application as an aggregator
    Start {
        /// Testing only: A path to write the latest pubkey to
        #[arg(short = 'k', long = "pubkey-write-path")]
        pubkey_write_path: Option<String>,

        /// Testing only: A path to write the latest plaintexts to
        #[arg(short, long = "plaintext-write-path")]
        plaintext_write_path: Option<String>,
    },
}

pub async fn execute(command: AggregatorCommands, config_path: Option<&str>) -> Result<()> {
    match command {
        AggregatorCommands::Start {
            pubkey_write_path,
            plaintext_write_path,
        } => {
            start::execute(
                config_path,
                pubkey_write_path.as_deref(),
                plaintext_write_path.as_deref(),
            )
            .await?
        }
    };

    Ok(())
}
