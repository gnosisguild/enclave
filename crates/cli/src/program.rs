use anyhow::Result;
use clap::Subcommand;
use e3_config::AppConfig;

#[derive(Subcommand, Debug)]
pub enum ProgramCommands {
    Listen,
}

pub async fn execute(command: ProgramCommands, _config: &AppConfig) -> Result<()> {
    match command {
        ProgramCommands::Listen => e3_support_scripts::program_listen().await?,
    };

    Ok(())
}
