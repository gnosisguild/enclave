use alloy::primitives::Address;
use clap::Parser;
use enclave_core::MainAggregator;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'n', long)]
    rpc: String,
    #[arg(short, long = "enclave-contract")]
    enclave_contract: String,
    #[arg(short, long = "registry-contract")]
    registry_contract: String,
    #[arg(short, long = "pubkey-write-path")]
    pubkey_write_path: Option<String>,
}

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("LAUNCHING AGGREGATOR");
    let registry_contract =
        Address::parse_checksummed(&args.registry_contract, None).expect("Invalid address");
    let enclave_contract =
        Address::parse_checksummed(&args.enclave_contract, None).expect("Invalid address");
    let (_, handle) = MainAggregator::attach(
        &args.rpc,
        enclave_contract,
        registry_contract,
        args.pubkey_write_path.as_deref(),
    )
    .await;
    let _ = tokio::join!(handle);
    Ok(())
}
