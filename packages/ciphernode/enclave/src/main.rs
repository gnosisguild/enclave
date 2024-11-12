use anyhow::Result;
use clap::Parser;
use commands::{aggregator, password, start, wallet, Commands};
use config::load_config;
use enclave_core::{get_tag, set_tag};
use tracing::instrument;
use tracing_subscriber::EnvFilter;
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

#[derive(Parser, Debug)]
#[command(name = "enclave")]
#[command(about = "A CLI for interacting with Enclave the open-source protocol for Encrypted Execution Environments (E3)", long_about = None)]
pub struct Cli {
    /// Path to config file
    #[arg(long, global = true)]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true)]
    tag: Option<String>,
}

impl Cli {
    #[instrument(skip(self),fields(id = get_tag()))]
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

    pub fn get_tag(&self) -> String {
        if let Some(tag) = self.tag.clone() {
            tag
        } else {
            "default".to_string()
        }
    }
}

#[actix::main]
pub async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        // .with_env_filter("error")
        // .with_env_filter("[app{id=cn1}]=info")
        // .with_env_filter("[app{id=cn2}]=info,libp2p_mdns::behaviour=error")
        // .with_env_filter("[app{id=cn3}]=info")
        // .with_env_filter("[app{id=cn4}]=info")
        // .with_env_filter("[app{id=ag}]=info")
        .init();
    let cli = Cli::parse();

    // Set the tag for all future traces
    if let Err(err) = set_tag(cli.get_tag()) {
        eprintln!("{}", err);
    }

    // Execute the cli
    if let Err(err) = cli.execute().await {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
