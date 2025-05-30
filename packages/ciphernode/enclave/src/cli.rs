use crate::helpers::telemetry::setup_tracing;
use crate::net;
use crate::net::NetCommands;
use crate::nodes::{self, NodeCommands};
use crate::password::PasswordCommands;
use crate::start;
use crate::wallet::WalletCommands;
use crate::{init, password, wallet};
use anyhow::{bail, Result};
use clap::{command, ArgAction, Parser, Subcommand};
use config::validation::ValidUrl;
use config::{load_config, AppConfig};
use enclave_core::helpers::datastore::close_all_connections;
use tracing::{info, instrument, Level};

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

    /// Set the Open Telemetry collector grpc endpoint. Eg. http://localhost:4317
    #[arg(long = "otel", global = true)]
    pub otel: Option<ValidUrl>,
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

    #[instrument(skip_all)]
    pub async fn execute(self) -> Result<()> {
        // Attempt to load the config, but only treat “not found” as
        // the trigger for the init flow.  All other errors bubble up.
        let config = match self.load_config() {
            Ok(cfg) => cfg,
            // If the file truly doesn’t exist, fall back to init
            Err(e)
                if matches!(
                    e.downcast_ref::<std::io::Error>(),
                    Some(ioe) if ioe.kind() == std::io::ErrorKind::NotFound
                ) =>
            {
                // Existing init branch
                match self.command {
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
                    _ => bail!(
                        "Configuration file not found. Have you created `enclave.config.yaml` in your project?"
                    ),
                };
                return Ok(());
            }
            // Any other error is fatal
            Err(e) => return Err(e),
        };

        setup_tracing(&config, self.log_level())?;
        info!("Config loaded from: {:?}", config.config_file());

        if config.autopassword() {
            enclave_core::password::set::autopassword(&config).await?;
        }

        if config.autonetkey() {
            enclave_core::net::keypair::generate::autonetkey(&config).await?;
        }

        if config.autowallet() {
            enclave_core::wallet::set::autowallet(&config).await?;
        }

        match self.command {
            Commands::Start { peers } => start::execute(config, peers).await?,
            Commands::Init { .. } => {
                bail!("Cannot run `enclave init` when a configuration exists.");
            }
            Commands::Nodes { command } => {
                nodes::execute(
                    command,
                    &config,
                    self.verbose,
                    self.config,
                    self.otel.clone().map(Into::into),
                )
                .await?
            }
            Commands::Password { command } => password::execute(command, &config).await?,
            Commands::Wallet { command } => wallet::execute(command, config).await?,
            Commands::Net { command } => net::execute(command, &config).await?,
        }

        close_all_connections();

        Ok(())
    }

    pub fn load_config(&self) -> Result<AppConfig> {
        let config = load_config(
            &self.name(),
            self.config.clone(),
            self.otel.clone().map(Into::into),
        )?;
        Ok(config)
    }

    pub fn name(&self) -> String {
        // If no name is provided assume we are working with the default node
        self.name.clone().unwrap_or("_default".to_string())
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the application
    Start {
        #[arg(
            long = "peer",
            action = clap::ArgAction::Append,
            value_name = "PEER",
            help = "Sets a peer URL",
        )]
        peers: Vec<String>,
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

    /// Manage multiple node processes together as a set
    Nodes {
        #[command(subcommand)]
        command: NodeCommands,
    },
}
