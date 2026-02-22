// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::owo;
use anyhow::{anyhow, Result};
use e3_config::{AppConfig, NodeRole};
use e3_entrypoint::helpers::listen_for_shutdown;
use tracing::{info, instrument};

#[instrument(skip_all)]
pub async fn execute(mut config: AppConfig, peers: Vec<String>) -> Result<()> {
    owo();

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

    info!(
        "LAUNCHING CIPHERNODE: ({}/{}/{})",
        config.name(),
        node.address,
        node.peer_id
    );

    tokio::spawn(listen_for_shutdown(node)).await?;

    Ok(())
}
