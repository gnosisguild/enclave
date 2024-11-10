use anyhow::*;
use config::AppConfig;
use enclave_node::{listen_for_shutdown, setup_aggregator};
use tracing::info;

use crate::owo;

pub async fn execute(
    config: AppConfig,
    pubkey_write_path: Option<&str>,
    plaintext_write_path: Option<&str>,
) -> Result<()> {
    owo();

    let (bus, handle, peer_id) =
        setup_aggregator(config, pubkey_write_path, plaintext_write_path).await?;

    info!("LAUNCHING AGGREGATOR {}", peer_id);
    tokio::spawn(listen_for_shutdown(bus.into(), handle));

    std::future::pending::<()>().await;

    Ok(())
}
