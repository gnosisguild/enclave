use anyhow::Result;
use clap::Subcommand;
use e3_config::AppConfig;

#[derive(Subcommand, Debug)]
pub enum ProgramCommands {
    /// Start the program
    Start {
        /// Run the program in Dev Mode. Dev Mode is when the program will run without any proving
        /// backend at all. Your program will simply execute without being verified.
        #[arg(long)]
        dev: bool,
    },

    /// Compile the program code
    Compile {
        /// Compile the program in Dev Mode.
        #[arg(long)]
        dev: bool,
    },

    /// Get a shell into the docker environment that the program runs in
    Shell,

    /// Commands to manage the program compilation cache
    Cache {
        #[command(subcommand)]
        command: ProgramCacheCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProgramCacheCommands {
    /// Purge program compilation caches. Will make program compilation take longer.
    Purge,
}

pub async fn execute(command: ProgramCommands, config: &AppConfig) -> Result<()> {
    match command {
        ProgramCommands::Start { dev } => {
            e3_support_scripts::program_start(config.program().clone(), dev).await?
        }
        ProgramCommands::Compile { dev } => {
            e3_support_scripts::program_compile(config.program().clone(), dev).await?
        }
        ProgramCommands::Shell => e3_support_scripts::program_shell().await?,
        ProgramCommands::Cache { command } => match command {
            ProgramCacheCommands::Purge => e3_support_scripts::program_cache_purge().await?,
        },
    };

    Ok(())
}
