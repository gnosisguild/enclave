// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::*;
use clap::Subcommand;
use e3_config::AppConfig;
use zeroize::Zeroizing;

use crate::{helpers::ensure_hex_zeroizing, wallet_get, wallet_set};

#[derive(Subcommand, Debug)]
pub enum WalletCommands {
    /// Set wallet private key
    Set {
        /// The private key - note we are leaving as hex string as it is easier to manage with
        /// the allow Signer coercion
        #[arg(long = "private-key", value_parser = ensure_hex_zeroizing)]
        private_key: Option<Zeroizing<String>>,
    },
    /// Get your wallet address
    Get,
}

pub async fn execute(command: WalletCommands, config: AppConfig) -> Result<()> {
    match command {
        WalletCommands::Set { private_key } => wallet_set::execute(&config, private_key).await?,
        WalletCommands::Get => wallet_get::execute(&config).await?,
    };

    Ok(())
}
