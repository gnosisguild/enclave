use e3_console::{log, Console};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use serde::Serialize;
use tokio::net::UnixStream;

const SOCKET_PATH: &str = "/tmp/enclave.sock";

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
