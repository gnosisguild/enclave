// SPDX-License-Identifier: LGPL-2.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_console::{log, Console};
use serde::Serialize;
use std::future::Future;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tracing::error;

pub const TCP_PORT: u16 = 50505;
const TCP_ADDRESS: &str = "127.0.0.1"; // using localhost specifically so that it is not mounted
                                       // externally. We might change this if we need to control
                                       // externally and add authentication or TLS

pub async fn connect_socket() -> Option<TcpStream> {
    let addr = format!("{}:{}", TCP_ADDRESS, TCP_PORT);
    TcpStream::connect(addr).await.ok()
}

pub async fn run_on_socket<T: Serialize>(
    out: Console,
    stream: TcpStream,
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

pub async fn start_socket_server<F, Fut>(handler: F)
where
    F: Fn(TcpStream) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + 'static,
{
    let addr = format!("{}:{}", TCP_ADDRESS, TCP_PORT);
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind socket: {e}");
            return;
        }
    };

    let handler = Arc::new(handler);
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let handler = Arc::clone(&handler);

                tokio::task::spawn_local(async move {
                    if let Err(e) = handler(stream).await {
                        error!("Connection error: {e}");
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}
