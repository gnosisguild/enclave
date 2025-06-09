use anyhow::Result;
use clap::Subcommand;
use e3_config::AppConfig;

#[derive(Subcommand, Debug)]
pub enum ProgramCommands {
    /// Start the FHE computation service with webhook callback
    Start {
        /// JSON RPC server URL to call when computation is complete
        #[arg(long)]
        json_rpc_server: String,

        /// Chain configuration to use
        #[arg(long)]
        chain: String,
    },

    /// Compile the program code
    Compile,
}

pub async fn execute(command: ProgramCommands, config: &AppConfig) -> Result<()> {
    match command {
        ProgramCommands::Start {
            json_rpc_server,
            chain,
        } => {
            e3_support_app::start_with_webhook(&json_rpc_server, &chain).await?;
        }
        ProgramCommands::Compile => e3_support_scripts::program_compile().await?,
    };

    Ok(())
}
