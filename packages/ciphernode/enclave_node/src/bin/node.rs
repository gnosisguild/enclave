use alloy::primitives::Address;
use clap::Parser;
use enclave_node::MainCiphernode;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'a', long)]
    address: String,
    #[arg(short='r', long)]
    rpc: String,
    #[arg(short = 'e', long = "enclave-contract")]
    enclave_contract: String,
    #[arg(short = 'c', long = "registry-contract")]
    registry_contract: String
}

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let address = Address::parse_checksummed(&args.address, None).expect("Invalid address");
    println!("LAUNCHING CIPHERNODE: ({})", address);
    let registry_contract = Address::parse_checksummed(&args.registry_contract, None).expect("Invalid address");
    let enclave_contract = Address::parse_checksummed(&args.enclave_contract, None).expect("Invalid address");
    let (_, handle) = MainCiphernode::attach(address, &args.rpc, enclave_contract, registry_contract).await;
    let _ = tokio::join!(handle);
    Ok(())
}
