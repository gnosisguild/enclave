// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::cli::{Cli, SerializedCli};
use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{error, info};

pub const SOCKET_PATH: &str = "/tmp/enclave.sock";

pub async fn start_socket_server() -> Result<()> {
    println!("starting socket server...");
    if Path::new(SOCKET_PATH).exists() {
        fs::remove_file(SOCKET_PATH)?;
    }

    let listener = UnixListener::bind(SOCKET_PATH)?;
    info!("Socket server listening on {}", SOCKET_PATH);

    let mut sigterm = signal(SignalKind::terminate())?;

    loop {
        select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        if let Err(e) = handle_connection(stream).await {
                            error!("Error handling connection: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Accept error: {}", e);
                    }
                }
            }
            _ = sigterm.recv() => {
                info!("SIGTERM received, stopping socket server");
                break;
            }
        }
    }

    if Path::new(SOCKET_PATH).exists() {
        fs::remove_file(SOCKET_PATH)?;
    }

    Ok(())
}

async fn handle_connection(stream: UnixStream) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    let Some(line) = lines.next_line().await? else {
        return Ok(());
    };

    let serialized: SerializedCli =
        serde_json::from_str(&line).map_err(|e| anyhow!("Failed to parse command: {}", e))?;

    let cli: Cli = serialized.try_into()?;

    match cli.execute().await {
        Ok(()) => {}
        Err(e) => {
            writer.write_all(b"Error: ").await?;
            writer.write_all(e.to_string().as_bytes()).await?;
            writer.write_all(b"\n").await?;
        }
    }

    writer.shutdown().await?;
    Ok(())
}
