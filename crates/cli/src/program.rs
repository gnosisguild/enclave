use anyhow::Result;
use clap::Subcommand;
use e3_config::AppConfig;
use e3_support_scripts::{SupportArgs, SupportConfig};

#[derive(Subcommand, Debug)]
pub enum ProgramCommands {
    /// Start the program
    Start {
        #[arg(long, short)]
        bonsai_api_key: Option<String>,
    },

    /// Compile the program code
    Compile,
}

pub async fn execute(command: ProgramCommands, _config: &AppConfig) -> Result<()> {
    match command {
        ProgramCommands::Start { bonsai_api_key } => {
            let support_args = match bonsai_api_key {
                Some(api_key) => SupportArgs::BonsaiCredentials {
                    api_key,
                    api_url: "https://api.bonsai.xyz".to_string(),
                },
                None => SupportArgs::DevMode,
            };
            e3_support_scripts::program_start(support_args).await?
        }
        ProgramCommands::Compile => e3_support_scripts::program_compile().await?,
    };

    Ok(())
}
