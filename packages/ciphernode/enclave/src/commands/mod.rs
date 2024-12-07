pub mod aggregator;
pub mod init;
pub mod net;
pub mod password;
pub mod start;
pub mod wallet;

use self::password::PasswordCommands;
use aggregator::AggregatorCommands;
use clap::Subcommand;
use net::NetCommands;
use wallet::WalletCommands;

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
        /// Testing only: A path to write the latest pubkey to
        #[arg(long = "rpc-url")]
        rpc_url: Option<String>,

        /// Testing only: A path to write the latest plaintexts to
        #[arg(long = "eth-address")]
        eth_address: Option<String>,

        /// The password
        #[arg(short, long)]
        password: Option<String>,

        /// Skip asking for eth
        #[arg(long = "skip-eth")]
        skip_eth: bool,
    },
}
