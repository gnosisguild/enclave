// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::owo;
use crate::socket_server::start_socket_server;
use anyhow::Result;
use e3_config::{AppConfig, NodeRole};
use e3_entrypoint::helpers::listen_for_shutdown;
use std::thread;
use tokio::{runtime::Builder, task::LocalSet};
use tracing::info;

pub async fn execute(mut config: AppConfig, peers: Vec<String>) -> Result<()> {
    owo();

    let socket_server = thread::spawn(move || {
        let rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create socket server runtime");
        rt.block_on(LocalSet::new().run_until(start_socket_server()))
    });
    config.add_peers(peers);
    println!("starting...");
    let node = match config.role() {
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

        NodeRole::Ciphernode => e3_entrypoint::start::start::execute(&config).await?,
    };
    println!("launched...");

    info!(
        "LAUNCHING CIPHERNODE: ({}/{}/{})",
        config.name(),
        node.address,
        node.peer_id
    );

    let node = tokio::spawn(listen_for_shutdown(node));

    node.await?;

    drop(socket_server.join());

    Ok(())
}
