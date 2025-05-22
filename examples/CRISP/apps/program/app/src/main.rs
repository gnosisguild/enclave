mod database;
use clap::Parser;
use database::SledDB;
use enclave_sdk::evm::events::E3Activated;
use enclave_sdk::indexer::EnclaveIndexer;
use eyre::{OptionExt, Result};

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
    let indexer = EnclaveIndexer::new(&cli.ws_url, &cli.contract_address, store).await?;
    let mut listener = indexer.get_listener();
    let store = indexer.get_store();
    listener
        .add_event_handler(move |event: E3Activated| {
            //
            async move {
                //
                Ok(())
            }
        })
        .await;
    let handler = indexer.start()?;
    handler.await?;
    Ok(())
}
