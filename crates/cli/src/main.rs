// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use clap::Parser;
use cli::Cli;
use tracing::info;

mod ciphernode;
mod cli;
pub mod helpers;
mod init;
mod net;
mod nodes;
mod nodes_daemon;
mod nodes_down;
mod nodes_ps;
mod nodes_purge;
mod nodes_restart;
mod nodes_start;
mod nodes_status;
mod nodes_stop;
mod nodes_up;
pub mod noir;
mod password;
mod password_delete;
mod password_set;
mod print_env;
mod program;
mod purge_all;
mod rev;
mod start;
mod wallet;
mod wallet_get;
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
