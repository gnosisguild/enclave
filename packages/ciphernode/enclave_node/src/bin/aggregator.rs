use alloy::primitives::Address;
use clap::Parser;
use enclave_core::MainAggregator;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'r', long)]
    rpc: String,
    #[arg(short = 'e', long = "enclave-contract")]
    enclave_contract: String,
    #[arg(short = 'c', long = "registry-contract")]
    registry_contract: String,
    #[arg(short = 'f', long = "registry-filter-contract")]
    registry_filter_contract: String,
    #[arg(short = 'p', long = "pubkey-write-path")]
    pubkey_write_path: Option<String>,
    #[arg(short = 't', long = "plaintext-write-path")]
    plaintext_write_path: Option<String>,
}

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("LAUNCHING AGGREGATOR");
    let registry_contract =
        Address::parse_checksummed(&args.registry_contract, None).expect("Invalid address");
    let registry_filter_contract = Address::parse_checksummed(
        &args.registry_filter_contract,
        None,
    )
    .expect("Invalid address");
    let enclave_contract =
        Address::parse_checksummed(&args.enclave_contract, None).expect("Invalid address");
    let (_, handle) = MainAggregator::attach(
        &args.rpc,
        enclave_contract,
        registry_contract,
        registry_filter_contract,
        args.pubkey_write_path.as_deref(),
        args.plaintext_write_path.as_deref()
    )
    .await;
    let _ = tokio::join!(handle);
    Ok(())
}
