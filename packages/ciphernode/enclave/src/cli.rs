use crate::net;
use crate::net::NetCommands;
use crate::password::PasswordCommands;
use crate::wallet::WalletCommands;
use crate::{aggregator, init, password, wallet};
use crate::{aggregator::AggregatorCommands, start};
use anyhow::Result;
use clap::{command, Parser, Subcommand};
use config::load_config;
use tracing::instrument;

#[derive(Parser, Debug)]
#[command(name = "enclave")]
#[command(about = "A CLI for interacting with Enclave the open-source protocol for Encrypted Execution Environments (E3)", long_about = None)]
pub struct Cli {
    /// Path to config file
    #[arg(long, global = true)]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true)]
    tag: Option<String>,
}

impl Cli {
    #[instrument(skip(self))]
    pub async fn execute(self) -> Result<()> {
        let config_path = self.config.as_deref();
        let config = load_config(config_path)?;

        match self.command {
            Commands::Start => start::execute(config).await?,
            Commands::Init {
                rpc_url,
                eth_address,
                password,
                skip_eth,
                net_keypair,
                generate_net_keypair,
            } => {
                init::execute(
                    rpc_url,
                    eth_address,
                    password,
                    skip_eth,
                    net_keypair,
                    generate_net_keypair,
                )
                .await?
            }
            Commands::Password { command } => password::execute(command, &config).await?,
            Commands::Aggregator { command } => aggregator::execute(command, config).await?,
            Commands::Wallet { command } => wallet::execute(command, config).await?,
            Commands::Net { command } => net::execute(command, &config).await?,
        }

        Ok(())
    }

    pub fn get_tag(&self) -> String {
        if let Some(tag) = self.tag.clone() {
            tag
        } else {
            "default".to_string()
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the application
    Start,

    /// Aggregator node management commands
    Aggregator {
        #[command(subcommand)]
        command: AggregatorCommands,
    },

    /// Password management commands
    Password {
        #[command(subcommand)]
        command: PasswordCommands,
    },

    /// Wallet management commands
    Wallet {
        #[command(subcommand)]
        command: WalletCommands,
    },

    /// Networking related commands
    Net {
        #[command(subcommand)]
        command: NetCommands,
    },

    Init {
        /// An rpc url for enclave to connect to
        #[arg(long = "rpc-url")]
        rpc_url: Option<String>,

        /// An Ethereum address that enclave should use to identify the node
        #[arg(long = "eth-address")]
        eth_address: Option<String>,

        /// The password
        #[arg(short, long)]
        password: Option<String>,

        /// Skip asking for eth
        #[arg(long = "skip-eth")]
        skip_eth: bool,

        /// The network private key (ed25519)
        #[arg(long = "net-keypair")]
        net_keypair: Option<String>,

        /// Generate a new network keypair
        #[arg(long = "generate-net-keypair")]
        generate_net_keypair: bool,
    },
}
