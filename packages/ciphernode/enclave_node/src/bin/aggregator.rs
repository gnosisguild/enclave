use alloy::primitives::Address;
use clap::Parser;
use enclave_core::MainAggregator;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short='n', long)]
    rpc: String,
    #[arg(short, long="registry-contract")]
    registry_contract: String
}

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("LAUNCHING AGGREGATOR");
    let registry_contract =
        Address::parse_checksummed(&args.registry_contract, None).expect("Invalid address");
    let (_, handle) = MainAggregator::attach(&args.rpc, registry_contract).await;
    let _ = tokio::join!(handle);
    Ok(())
}
