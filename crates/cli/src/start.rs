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
pub async fn execute(
    mut config: AppConfig,
    peers: Vec<String>,
    experimental_trbfv: bool,
) -> Result<()> {
    owo();

    let Some(address) = config.address() else {
        return Err(anyhow!("You must provide an address"));
    };

    // add cli peers to the config
    config.add_peers(peers);

    let (bus, handle, peer_id) = match config.role() {
        // Launch in aggregator configuration
        NodeRole::Aggregator {
            pubkey_write_path,
            plaintext_write_path,
        } => {
            e3_entrypoint::start::aggregator_start::execute(
                &config,
                pubkey_write_path,
                plaintext_write_path,
                experimental_trbfv,
            )
            .await?
        }

        // Launch in ciphernode configuration
        NodeRole::Ciphernode => {
            e3_entrypoint::start::start::execute(&config, address, experimental_trbfv).await?
        }
    };

    info!(
        "LAUNCHING CIPHERNODE: ({}/{}/{})",
        config.name(),
        address,
        peer_id
    );

    tokio::spawn(listen_for_shutdown(bus.into(), handle)).await?;

    Ok(())
}
