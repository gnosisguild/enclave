mod database;
use chrono::Utc;
use clap::Parser;
use database::SledDB;
use enclave_sdk::evm::events::{E3Activated, InputPublished};
use enclave_sdk::indexer::{get_e3, EnclaveIndexer};
use eyre::{OptionExt, Result};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep_until, Instant};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    ws_url: String,
    #[arg(short, long)]
    contract_address: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let pathdb = std::env::current_dir()?.join("database/program");
    let store = SledDB::new(&pathdb.to_str().ok_or_eyre("Bad path provided")?)?;
    let mut indexer = EnclaveIndexer::new(&cli.ws_url, &cli.contract_address, store).await?;
    indexer
        .add_event_handler(move |event: E3Activated, store| {
            //
            let expiration = event.expiration.to::<u64>();
            async move {
                let expiration = Instant::now()
                    + (UNIX_EPOCH + Duration::from_secs(expiration))
                        .duration_since(SystemTime::now())
                        .unwrap_or_else(|_| Duration::ZERO);
                sleep_until(expiration).await;
                Ok(())
            }
        })
        .await;

    indexer
        .add_event_handler(move |event: InputPublished, store| {})
        .await;

    let handler = indexer.start()?;
    handler.await?;
    Ok(())
}
