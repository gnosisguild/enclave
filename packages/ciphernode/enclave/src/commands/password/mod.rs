mod create;
mod delete;
mod overwrite;
use anyhow::*;
use clap::Subcommand;
use config::AppConfig;

#[derive(Subcommand, Debug)]
pub enum PasswordCommands {
    /// Create a new password
    Create {
        /// The new password
        #[arg(short, long)]
        password: Option<String>,
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

pub async fn execute(command: PasswordCommands, config: AppConfig) -> Result<()> {
    match command {
        PasswordCommands::Create { password } => create::execute(&config, password).await?,
        PasswordCommands::Delete => delete::execute(&config).await?,
        PasswordCommands::Overwrite { password } => overwrite::execute(&config, password).await?,
    };

    Ok(())
}
