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

    info!("LAUNCHING AGGREGATOR");

    let (bus, handle) = setup_aggregator(config, pubkey_write_path, plaintext_write_path).await?;

    tokio::spawn(listen_for_shutdown(bus.into(), handle));

    std::future::pending::<()>().await;

    Ok(())
}
