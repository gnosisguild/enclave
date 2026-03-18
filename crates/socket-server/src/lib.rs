// SPDX-License-Identifier: LGPL-2.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_console::{log, Console};
use serde::Serialize;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::error;

pub const SOCKET_PATH: &str = "/tmp/enclave.sock";

pub async fn connect_socket() -> Option<UnixStream> {
    if !Path::new(SOCKET_PATH).exists() {
        return None;
    }
    UnixStream::connect(SOCKET_PATH).await.ok()
}

pub async fn run_on_socket<T: Serialize>(
    out: Console,
    stream: UnixStream,
    cli: T,
) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let payload = serde_json::to_string(&cli)?;
    writer.write_all(payload.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.shutdown().await?;

    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await? {
        log!(out, "{}", line);
    }

    Ok(())
}

pub fn remove_socket() -> Result<()> {
    std::fs::remove_file(SOCKET_PATH)?;
    Ok(())
}

pub async fn start_socket_server<F, Fut>(handler: F)
where
    F: Fn(UnixStream) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + 'static,
{
    let _ = std::fs::remove_file(SOCKET_PATH);
    let listener = match tokio::net::UnixListener::bind(SOCKET_PATH) {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind socket: {e}");
            return;
        }
    };

    let handler = Arc::new(handler);

    while let Ok((stream, _)) = listener.accept().await {
        let handler = Arc::clone(&handler);
        tokio::task::spawn_local(async move {
            if let Err(e) = handler(stream).await {
                error!("Connection error: {e}");
            }
        });
    }
}
