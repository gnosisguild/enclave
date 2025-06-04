use anyhow::Result;
use clap::Subcommand;
use e3_config::AppConfig;

#[derive(Subcommand, Debug)]
pub enum ProgramCommands {
    /// Listen for blockchain events and trigger a computation after an E3Request has expired
    Listen {
        /// Webhook to trigger upon execution completion
        #[arg(long)]
        json_rpc_server: String,

        /// Webhook to trigger upon execution completion
        #[arg(long)]
        chain: String,
    },
}

pub async fn execute(command: ProgramCommands, config: &AppConfig) -> Result<()> {
    match command {
        ProgramCommands::Listen {
            json_rpc_server,
            chain,
        } => {
            e3_program_listener::execute(config, &chain, &json_rpc_server).await?;
        }
    };

    Ok(())
}
