use clap::Parser;
use enclave_core::MainAggregator;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    address: String,
}

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (_, handle) = MainAggregator::attach().await;
    let _ = tokio::join!(handle);
    Ok(())
}
