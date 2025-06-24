use anyhow::Result;
use clap::Subcommand;
use e3_config::AppConfig;

#[derive(Subcommand, Debug)]
pub enum ProgramCommands {
    /// Start the program
    Start,

    /// Compile the program code
    Compile,

    /// Get a shell into the docker environment that the program runs in
    Shell,
}

pub async fn execute(command: ProgramCommands, config: &AppConfig) -> Result<()> {
    match command {
        ProgramCommands::Start => e3_support_scripts::program_start(config.program()).await?,
        ProgramCommands::Compile => e3_support_scripts::program_compile().await?,
        ProgramCommands::Shell => e3_support_scripts::program_shell().await?,
    };

    Ok(())
}
