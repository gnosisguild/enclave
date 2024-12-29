use clap::Parser;
use cli::Cli;
use enclave_core::set_tag;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod aggregator;
mod aggregator_start;
mod cli;
pub mod helpers;
mod init;
pub mod net;
mod net_generate;
mod net_purge;
mod net_set;
mod password;
mod password_create;
mod password_delete;
mod password_overwrite;
mod start;
mod wallet;
mod wallet_set;

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

    info!("COMPILATION ID: '{}'", helpers::compile_id::generate_id());

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
