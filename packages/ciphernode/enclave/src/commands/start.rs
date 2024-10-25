use alloy::primitives::Address;
use anyhow::{Context, Result};
use config::load_config;
use enclave_node::{listen_for_shutdown, MainCiphernode};
use tracing::info;

use crate::owo;

pub async fn execute(config_path:Option<&str>, address:&str) -> Result<()> {

    owo();

    let address = Address::parse_checksummed(&address, None).context("Invalid address")?;
    info!("LAUNCHING CIPHERNODE: ({})", address);
    let config = load_config(config_path)?;

    let (bus, handle) = MainCiphernode::attach(config, address).await?;

    tokio::spawn(listen_for_shutdown(bus.into(), handle));

    std::future::pending::<()>().await;

    Ok(())
}

