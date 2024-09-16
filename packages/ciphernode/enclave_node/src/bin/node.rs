use alloy_primitives::Address;
use clap::Parser;
use enclave_core::MainCiphernode;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    address: String,
}

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let address = Address::parse_checksummed(&args.address, None).expect("Invalid address");
    let (_, handle) = MainCiphernode::attach(address).await;
    let _ = tokio::join!(handle);
    Ok(())
}
