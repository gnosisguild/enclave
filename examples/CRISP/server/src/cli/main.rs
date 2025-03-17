mod config;
mod commands;

use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use reqwest::Client;

use config::CONFIG;
use crisp::logger::init_logger;
use log::info;
use commands::{
    activate_e3_round, decrypt_and_publish_result, initialize_crisp_round,
    participate_in_existing_round,
};

use once_cell::sync::Lazy;

use sled::Db;
use std::sync::Arc;
use tokio::sync::RwLock;

pub static CLI_DB: Lazy<Arc<RwLock<Db>>> = Lazy::new(|| {
    let pathdb = std::env::current_dir().unwrap().join("database/cli");
    Arc::new(RwLock::new(sled::open(pathdb).unwrap()))
});


#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_logger();

    let client = Client::new();

    let environment = select_environment()?;
    if environment != 0 {
        info!("Check back soon!");
        return Ok(());
    }

    let action = select_action()?;

    match action {
        0 => {
            initialize_crisp_round().await?;
        }
        1 => {
            activate_e3_round().await?;
        }
        2 => {
            participate_in_existing_round(&client).await?;
        }
        3 => {
            decrypt_and_publish_result(&client).await?;
        }
        _ => unreachable!(),
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
        "Activate an E3 round.",
        "Participate in an E3 round.",
        "Decrypt Ciphertext & Publish Results",
    ];
    Ok(FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Create a new CRISP round or participate in an existing round.")
        .default(0)
        .items(&selections[..])
        .interact()?)
}
