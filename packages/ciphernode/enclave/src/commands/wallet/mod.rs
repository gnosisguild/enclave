mod set;
use anyhow::*;
use clap::Subcommand;
use config::AppConfig;

#[derive(Subcommand)]
pub enum WalletCommands {
    /// Set a new Wallet Private Key
    Set {
        /// The new private key
        #[arg(long = "private-key")]
        private_key: String,
    },
    // /// Delete the current wallet
    // Delete,
}

pub async fn execute(command: WalletCommands, config: AppConfig) -> Result<()> {
    match command {
        WalletCommands::Set { private_key } => set::execute(&config, private_key).await?,
    };

    Ok(())
}
