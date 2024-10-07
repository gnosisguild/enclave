use clap::Parser;
use enclave::load_config;
use enclave_node::MainAggregator;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    pub config: String,

    // These are for testing and may be removed later
    // or put under a compile flag
    #[arg(short = 'k', long = "pubkey-write-path")]
    pub pubkey_write_path: Option<String>,
    #[arg(short, long = "plaintext-write-path")]
    pub plaintext_write_path: Option<String>,
}

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("LAUNCHING AGGREGATOR");
    let config = load_config(&args.config)?;
    let (_, handle) = MainAggregator::attach(
        config,
        args.pubkey_write_path.as_deref(),
        args.plaintext_write_path.as_deref(),
    )
    .await?;
    let _ = tokio::join!(handle);
    Ok(())
}
