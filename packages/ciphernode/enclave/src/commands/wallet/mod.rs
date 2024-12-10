mod set;
use anyhow::*;
use clap::Subcommand;
use config::AppConfig;

#[derive(Subcommand, Debug)]
pub enum WalletCommands {
    /// Set a new Wallet Private Key
    Set {
        /// The new private key - note we are leaving as hex string as it is easier to manage with
        /// the allow Signer coercion
        #[arg(long = "private-key", value_parser = ensure_hex)]
        private_key: Option<String>,
    },
}

fn ensure_hex(s: &str) -> Result<String> {
    if !s.starts_with("0x") {
        bail!("hex value must start with '0x'")
    }
    hex::decode(&s[2..])?;
    Ok(s.to_string())
}

pub async fn execute(command: WalletCommands, config: AppConfig) -> Result<()> {
    match command {
        WalletCommands::Set { private_key } => set::execute(&config, private_key).await?,
    };

    Ok(())
}
