pub mod start;
pub mod password;
pub mod aggregator;

use aggregator::AggregatorCommands;
use clap::Subcommand;
use self::password::PasswordCommands;

#[derive(Subcommand)]
pub enum Commands {
    /// Start the application
    Start {
        #[arg(long)]
        address: String,
    },

    Aggregator {
        #[command(subcommand)]
        command: AggregatorCommands,
    },
    
    /// Password management commands
    Password {
        #[command(subcommand)]
        command: PasswordCommands,
    },
}

