// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::path::Path;

use clap::Parser;
use cli::{Cli, SerializedCli};
use e3_utils::{colorize, Color};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::info;

pub mod ciphernode;
pub mod cli;
pub mod helpers;
pub mod init;
pub mod net;
pub mod net_get_peer_id;
pub mod nodes;
pub mod nodes_daemon;
pub mod nodes_down;
pub mod nodes_ps;
pub mod nodes_purge;
pub mod nodes_restart;
pub mod nodes_start;
pub mod nodes_status;
pub mod nodes_stop;
pub mod nodes_up;
pub mod noir;
pub mod password;
pub mod password_delete;
pub mod password_set;
pub mod print_env;
pub mod program;
pub mod purge_all;
pub mod rev;
pub mod socket_server;
pub mod start;
pub mod wallet;
pub mod wallet_get;
pub mod wallet_set;

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
use crate::socket_server::SOCKET_PATH;

async fn connect_socket() -> Option<UnixStream> {
    if !Path::new(SOCKET_PATH).exists() {
        return None;
    }
    UnixStream::connect(SOCKET_PATH).await.ok()
}

async fn run_on_socket<T>(cli: T, stream: UnixStream) -> anyhow::Result<()>
where
    T: TryInto<SerializedCli, Error = anyhow::Error>,
{
    let (reader, mut writer) = stream.into_split();
    let cli: SerializedCli = cli.try_into()?;
    let payload = serde_json::to_string(&cli)?;
    writer.write_all(payload.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.shutdown().await?;

    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await? {
        println!("{}", line);
    }

    Ok(())
}

#[actix::main]
pub async fn main() -> anyhow::Result<()> {
    info!("COMPILATION ID: '{}'", helpers::compile_id::generate_id());

    let cli = Cli::parse();

    if let Err(err) = if let Some(stream) = connect_socket().await {
        run_on_socket(cli, stream).await
    } else {
        cli.execute().await
    } {
        eprintln!("{}", colorize(err, Color::Red));
        std::process::exit(1);
    }

    Ok(())
}
