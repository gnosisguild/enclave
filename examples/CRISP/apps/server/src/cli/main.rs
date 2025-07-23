// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod commands;
mod config;

use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use reqwest::Client;

use commands::initialize_crisp_round;
use config::CONFIG;
use crisp::logger::init_logger;
use log::info;

use clap::{Parser, Subcommand};
use once_cell::sync::Lazy;
use sled::Db;
use std::sync::Arc;
use tokio::sync::RwLock;

pub static CLI_DB: Lazy<Arc<RwLock<Db>>> = Lazy::new(|| {
    let pathdb = std::env::current_dir().unwrap().join("database/cli");
    Arc::new(RwLock::new(sled::open(pathdb).unwrap()))
});

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional environment selection (default: 0)
    #[arg(short, long, default_value_t = 0)]
    environment: usize,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize new E3 round
    Init,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_logger();

    let client = Client::new();
    let cli = Cli::parse();

    if cli.environment != 0 {
        info!("Check back soon!");
        return Ok(());
    }

    match cli.command {
        Some(Commands::Init) => {
            initialize_crisp_round().await?;
        }
        None => {
            // Fall back to interactive mode if no command was specified
            let action = select_action()?;
            match action {
                0 => {
                    initialize_crisp_round().await?;
                }
                _ => unreachable!(),
            }
        }
    }

    Ok(())
}

fn select_environment() -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let selections = &["CRISP: Voting Protocol (ETH)", "More Coming Soon!"];
    Ok(FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Enclave (EEEE): Please choose the private execution environment you would like to run!")
        .default(0)
        .items(&selections[..])
        .interact()?)
}

fn select_action() -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let selections = &[
        "Initialize new E3 round.",
        // "Participate in an E3 round.",
        // "Activate an E3 round.",
        // "Decrypt Ciphertext & Publish Results",
    ];
    Ok(FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Create a new CRISP round or participate in an existing round.")
        .default(0)
        .items(&selections[..])
        .interact()?)
}
