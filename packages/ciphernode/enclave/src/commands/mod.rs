pub mod start;
pub mod password;
pub mod aggregator;
pub mod wallet;

use aggregator::AggregatorCommands;
use clap::Subcommand;
use wallet::WalletCommands;
use self::password::PasswordCommands;

#[derive(Subcommand)]
pub enum Commands {
    /// Start the application
    Start {
        #[arg(long)]
        address: String,
    },

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
        command: WalletCommands
    }
}

