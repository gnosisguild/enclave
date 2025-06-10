use anyhow::Result;
use clap::Subcommand;
use e3_config::AppConfig;

#[derive(Subcommand, Debug)]
pub enum ProgramCommands {
    /// Start the program
    Start,

    /// Compile the program code
    Compile,
}

pub async fn execute(command: ProgramCommands, _config: &AppConfig) -> Result<()> {
    match command {
        ProgramCommands::Start => e3_support_scripts::program_start().await?,
        ProgramCommands::Compile => e3_support_scripts::program_compile().await?,
    };

    Ok(())
}
