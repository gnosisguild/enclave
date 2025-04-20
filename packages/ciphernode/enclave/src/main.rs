use clap::Parser;
use cli::Cli;
use tracing::info;

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
mod swarm;
mod swarm_daemon;
mod swarm_down;
mod swarm_ps;
mod swarm_restart;
mod swarm_start;
mod swarm_status;
mod swarm_stop;
mod swarm_up;
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
    info!("COMPILATION ID: '{}'", helpers::compile_id::generate_id());

    // Execute the cli
    if let Err(err) = Cli::parse().execute().await {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
