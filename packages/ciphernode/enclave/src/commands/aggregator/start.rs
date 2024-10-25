use anyhow::*;
use config::load_config;
use enclave_node::{listen_for_shutdown, MainAggregator};
use tracing::info;

use crate::owo;

pub async fn execute(
    config_path:Option<&str>,
    pubkey_write_path: Option<&str>,
    plaintext_write_path: Option<&str>,
) -> Result<()> {
    owo();

    info!("LAUNCHING AGGREGATOR");

    let conf = load_config(config_path)?;
    let (bus, handle) = MainAggregator::attach(
        conf,
        pubkey_write_path,
        plaintext_write_path,
    )
    .await?;

    tokio::spawn(listen_for_shutdown(bus.into(), handle));

    std::future::pending::<()>().await;

    Ok(())
}
