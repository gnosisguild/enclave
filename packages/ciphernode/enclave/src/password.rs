use anyhow::*;
use clap::Subcommand;
use config::AppConfig;

use crate::{password_create, password_delete, password_overwrite};

#[derive(Subcommand, Debug)]
pub enum PasswordCommands {
    /// Create a new password
    Create {
        /// The new password
        #[arg(short, long)]
        password: Option<String>,

        #[arg(short, long)]
        overwrite: bool,
    },

    /// Delete the current password
    Delete,

    /// Overwrite the current password
    Overwrite {
        /// The new password
        #[arg(short, long)]
        password: Option<String>,
    },
}

pub async fn execute(command: PasswordCommands, config: &AppConfig) -> Result<()> {
    match command {
        PasswordCommands::Create {
            password,
            overwrite,
        } => password_create::execute(&config, password, overwrite).await?,
        PasswordCommands::Delete => password_delete::execute(&config).await?,
        PasswordCommands::Overwrite { password } => {
            password_overwrite::execute(&config, password).await?
        }
    };

    Ok(())
}
