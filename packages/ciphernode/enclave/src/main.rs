use clap::Parser;
use enclave::cli::Cli;

#[actix_rt::main]
pub async fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.execute().await {
        Ok(_) => (),
        Err(_) => println!("There was a problem running. Goodbye"),
    }
}
