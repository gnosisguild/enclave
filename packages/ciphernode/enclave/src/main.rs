use anyhow::Result;
use clap::Parser;
use commands::{aggregator, password, start, wallet, Commands};
use config::load_config;
pub mod commands;

const OWO: &str = r#"
      ___           ___           ___                         ___                         ___     
     /\__\         /\  \         /\__\                       /\  \          ___          /\__\    
    /:/ _/_        \:\  \       /:/  /                      /::\  \        /\  \        /:/ _/_   
   /:/ /\__\        \:\  \     /:/  /                      /:/\:\  \       \:\  \      /:/ /\__\  
  /:/ /:/ _/_   _____\:\  \   /:/  /  ___   ___     ___   /:/ /::\  \       \:\  \    /:/ /:/ _/_ 
 /:/_/:/ /\__\ /::::::::\__\ /:/__/  /\__\ /\  \   /\__\ /:/_/:/\:\__\  ___  \:\__\  /:/_/:/ /\__\
 \:\/:/ /:/  / \:\~~\~~\/__/ \:\  \ /:/  / \:\  \ /:/  / \:\/:/  \/__/ /\  \ |:|  |  \:\/:/ /:/  /
  \::/_/:/  /   \:\  \        \:\  /:/  /   \:\  /:/  /   \::/__/      \:\  \|:|  |   \::/_/:/  / 
   \:\/:/  /     \:\  \        \:\/:/  /     \:\/:/  /     \:\  \       \:\__|:|__|    \:\/:/  /  
    \::/  /       \:\__\        \::/  /       \::/  /       \:\__\       \::::/__/      \::/  /   
     \/__/         \/__/         \/__/         \/__/         \/__/        ~~~~           \/__/    
                                                                      
"#;

pub fn owo() {
    println!("\n\n\n\n\n{}", OWO);
    println!("\n\n\n\n");
}

#[derive(Parser)]
#[command(name = "enclave")]
#[command(about = "A CLI for interacting with Enclave the open-source protocol for Encrypted Execution Environments (E3)", long_about = None)]
pub struct Cli {
    /// Path to config file
    #[arg(long, global = true)]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn execute(self) -> Result<()> {
        let config_path = self.config.as_deref();

        let config = load_config(config_path)?;

        match self.command {
            Commands::Start => start::execute(config).await?,
            Commands::Password { command } => password::execute(command, config).await?,
            Commands::Aggregator { command } => aggregator::execute(command, config).await?,
            Commands::Wallet { command } => wallet::execute(command, config).await?,
        }

        Ok(())
    }
}

#[actix_rt::main]
pub async fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.execute().await {
        Ok(_) => (),
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    }
}
