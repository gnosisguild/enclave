use crate::helpers::telemetry::setup_tracing;
use crate::net;
use crate::net::NetCommands;
use crate::password::PasswordCommands;
use crate::wallet::WalletCommands;
use crate::{aggregator, init, password, wallet};
use crate::{aggregator::AggregatorCommands, start};
use anyhow::Result;
use clap::{command, ArgAction, Parser, Subcommand};
use config::{load_config_from_overrides, AppConfig, CliOverrides};
use tracing::Level;

#[derive(Parser, Debug)]
#[command(name = "enclave")]
#[command(about = "A CLI for interacting with Enclave the open-source protocol for Encrypted Execution Environments (E3)", long_about = None)]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, global = true)]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,

    /// Indicate error levels by adding additional `-v` arguments. Eg. `enclave -vvv` will give you
    /// trace level output
    #[arg(
        short,
        long,
        action = ArgAction::Count,
        global = true
    )]
    pub verbose: u8,

    /// Silence all output. This argument cannot be used alongside `-v`
    #[arg(
        short,
        long,
        action = ArgAction::SetTrue,
        conflicts_with = "verbose",
        global = true
    )]
    quiet: bool,

    /// The node name (used for logs and open telemetry)
    #[arg(long, global = true)]
    pub name: Option<String>,

    /// Set the Open Telemetry collector grpc endpoint. Eg. 127.0.0.1:4317
    #[arg(long = "otel", global = true)]
    pub otel: Option<std::net::SocketAddr>,
}

impl Cli {
    pub fn log_level(&self) -> Level {
        if self.quiet {
            Level::ERROR
        } else {
            match self.verbose {
                0 => Level::WARN,  //
                1 => Level::INFO,  // -v
                2 => Level::DEBUG, // -vv
                _ => Level::TRACE, // -vvv
            }
        }
    }

    pub async fn execute(self) -> Result<()> {
        let config = self.load_config()?;
        setup_tracing(&config, self.log_level())?;

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

    fn load_config(&self) -> Result<AppConfig> {
        load_config_from_overrides(CliOverrides {
            config: self.config.clone(),
            name: self.name.clone(),
            otel: self.otel,
        })
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

    /// Initialize your ciphernode by setting up a configuration
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
