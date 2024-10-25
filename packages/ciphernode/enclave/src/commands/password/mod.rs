mod set;
use anyhow::*;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum PasswordCommands {
    /// Set a new password
    Set {
        /// The new password
        #[arg(short, long)]
        value: String,
    },
}

pub async fn execute(command: PasswordCommands, config_path: Option<&str>) -> Result<()> {
    if let Some(path) = config_path {
        println!("Using config from: {}", path);
    }

    match command {
        PasswordCommands::Set { value } => set::execute(value) 
    };

    Ok(())
}
