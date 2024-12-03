pub mod aggregator;
pub mod net;
pub mod password;
pub mod start;
pub mod wallet;
pub mod init;

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

    Init
}
