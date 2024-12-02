pub mod aggregator;
pub mod password;
pub mod start;
pub mod wallet;
pub mod p2p;

use self::password::PasswordCommands;
use aggregator::AggregatorCommands;
use clap::Subcommand;
use p2p::P2pCommands;
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

    /// Peer related commands
    P2p {
        #[command(subcommand)]
        command: P2pCommands
    }
}
