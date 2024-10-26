use alloy::primitives::Address;
use anyhow::{Context, Result};
use config::AppConfig;
use enclave_node::{listen_for_shutdown, setup_ciphernode};
use tracing::info;

use crate::owo;

pub async fn execute(config: AppConfig, address: &str) -> Result<()> {
    owo();

    let address = Address::parse_checksummed(&address, None).context("Invalid address")?;

    info!("LAUNCHING CIPHERNODE: ({})", address);

    let (bus, handle) = setup_ciphernode(config, address).await?;

    tokio::spawn(listen_for_shutdown(bus.into(), handle));

    std::future::pending::<()>().await;

    Ok(())
}
