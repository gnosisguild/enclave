// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{error, info};

pub const SOCKET_PATH: &str = "/tmp/enclave.sock";

pub async fn start_socket_server() -> Result<()> {
    if Path::new(SOCKET_PATH).exists() {
        std::fs::remove_file(SOCKET_PATH)?;
    }

    let listener = UnixListener::bind(SOCKET_PATH)?;
    info!("Socket server listening on {}", SOCKET_PATH);

    let mut sigterm = signal(SignalKind::terminate())?;

    loop {
        select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream).await {
                                error!("Error handling connection: {}", e);
                            }
                        });
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
        std::fs::remove_file(SOCKET_PATH)?;
    }

    Ok(())
}

async fn handle_connection(mut stream: tokio::net::UnixStream) -> Result<()> {
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                error!("Read error: {}", e);
                break;
            }
        }
    }
    stream.shutdown().await?;
    Ok(())
}
