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

    Cache {
        #[command(subcommand)]
        command: ProgramCacheCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProgramCacheCommands {
    /// Purge all caches
    Purge,
}

pub async fn execute(command: ProgramCommands, config: &AppConfig) -> Result<()> {
    match command {
        ProgramCommands::Start => e3_support_scripts::program_start(config.program()).await?,
        ProgramCommands::Compile => e3_support_scripts::program_compile().await?,
        ProgramCommands::Shell => e3_support_scripts::program_shell().await?,
        ProgramCommands::Cache { command } => match command {
            ProgramCacheCommands::Purge => e3_support_scripts::program_cache_purge().await?,
        },
    };

    Ok(())
}
