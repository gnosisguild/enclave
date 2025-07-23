// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::*;
use clap::Subcommand;
use e3_config::AppConfig;

use crate::{password_delete, password_set};

#[derive(Subcommand, Debug)]
pub enum PasswordCommands {
    /// Set (or overwrite) a password
    Set {
        /// The new password
        #[arg(short, long)]
        password: Option<String>,
    },

    /// Delete the current password
    Delete,
}

pub async fn execute(command: PasswordCommands, config: &AppConfig) -> Result<()> {
    match command {
        PasswordCommands::Set { password } => password_set::execute(&config, password).await?,
        PasswordCommands::Delete => password_delete::execute(&config).await?,
    };

    Ok(())
}
