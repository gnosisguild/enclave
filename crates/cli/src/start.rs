// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::time::Duration;

use crate::{
    cli::{Cli, RemoteCli},
    owo,
};
use anyhow::Result;
use e3_ciphernode_builder::CiphernodeHandle;
use e3_config::{AppConfig, NodeRole};
use e3_console::Console;
use e3_events::{prelude::*, Shutdown};
use e3_socket_server::start_socket_server;
use e3_utils::{colorize, Color};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::signal::unix::{signal, SignalKind};
use tracing::{error, info, instrument};

#[instrument(skip_all)]
pub async fn execute(mut config: AppConfig, peers: Vec<String>) -> Result<()> {
    // Register signal listeners immediately at startup
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    owo();
    launch_socket_server();

    let node = tokio::select! {
        // build the ciphernode and if it completes first return the result
        result = build_ciphernode(&mut config, peers) => result,
        // if the shutdown signal completes first then do shutdown without the node
        _ = &mut shutdown => {
            graceful_shutdown(None).await;
            return Ok(());
        }
    }?;

    info!(
        "LAUNCHING CIPHERNODE: ({}/{}/{})",
        config.name(),
        node.address,
        node.peer_id
    );

    shutdown.await;
    graceful_shutdown(Some(node)).await;

    Ok(())
}

/// Launch a socket server to read RemoteCli commands
pub fn launch_socket_server() {
    // Setup socket server for daemon
    tokio::task::spawn_local(start_socket_server(|stream| async move {
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        if let Some(line) = lines.next_line().await? {
            let (out, mut rx) = Console::channel();
            info!("CMD: {}", &colorize(&line, Color::Blue));
            let remote_cli: RemoteCli = serde_json::from_str(&line)?;
            let cli: Cli = remote_cli.try_into()?;
            cli.execute(out).await?;
            while let Some(msg) = rx.recv().await {
                writer.write_all(format!("{msg}\n").as_bytes()).await?;
            }
        }

        writer.shutdown().await?;
        Ok(())
    }));
}

pub async fn build_ciphernode(
    config: &mut AppConfig,
    peers: Vec<String>,
) -> Result<CiphernodeHandle> {
    // add cli peers to the config
    config.add_peers(peers);

    let node = match config.role() {
        // Launch in aggregator configuration
        NodeRole::Aggregator {
            pubkey_write_path,
            plaintext_write_path,
        } => {
            e3_entrypoint::start::aggregator_start::execute(
                &config,
                pubkey_write_path,
                plaintext_write_path,
            )
            .await?
        }

        // Launch in ciphernode configuration
        NodeRole::Ciphernode => e3_entrypoint::start::start::execute(&config).await?,
    };

    Ok(node)
}

pub fn shutdown_signal() -> impl std::future::Future<Output = ()> {
    let mut sigint =
        signal(SignalKind::interrupt()).expect("Failed to create SIGINT signal stream");
    let mut sigterm =
        signal(SignalKind::terminate()).expect("Failed to create SIGTERM signal stream");

    async move {
        tokio::select! {
            _ = sigint.recv() => info!("SIGINT received"),
            _ = sigterm.recv() => info!("SIGTERM received"),
        }
    }
}

pub async fn graceful_shutdown(node: Option<CiphernodeHandle>) {
    info!("initiating graceful shutdown...");

    if let Some(node) = node {
        if let Err(e) = node.bus.publish_without_context(Shutdown) {
            error!("Shutdown failed to publish! {e}");
        }
    }

    tokio::time::sleep(Duration::from_secs(2)).await;
    info!("Graceful shutdown complete");
}
