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

mod ciphernode;
mod cli;
pub mod helpers;
mod init;
mod net;
mod net_get_peer_id;
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
const SOCKET_PATH: &str = "/tmp/myapp.sock";

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
