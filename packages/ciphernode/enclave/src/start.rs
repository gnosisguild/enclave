use crate::owo;
use anyhow::{anyhow, Result};
use config::{AppConfig, NodeRole};
use enclave_core::{aggregator_start, helpers::listen_for_shutdown, start};
use tracing::{info, instrument};

#[instrument(skip_all)]
pub async fn execute(mut config: AppConfig, peers: Vec<String>) -> Result<()> {
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
        } => aggregator_start::execute(&config, pubkey_write_path, plaintext_write_path).await?,

        // Launch in ciphernode configuration
        NodeRole::Ciphernode => start::execute(&config, address).await?,
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
