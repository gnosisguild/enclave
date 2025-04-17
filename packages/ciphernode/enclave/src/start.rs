use crate::owo;
use anyhow::{anyhow, Result};
use config::{AppConfig, NodeRole};
use enclave_core::{aggregator_start, listen_for_shutdown, start};
use tracing::{info, instrument};

#[instrument(name = "app", skip_all)]
pub async fn execute(config: AppConfig) -> Result<()> {
    owo();
    let Some(address) = config.address() else {
        return Err(anyhow!("You must provide an address"));
    };

    let (bus, handle, peer_id) = match config.role() {
        NodeRole::Aggregator {
            pubkey_write_path,
            plaintext_write_path,
        } => aggregator_start::execute(&config, &pubkey_write_path, &plaintext_write_path).await?,
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
