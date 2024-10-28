use anyhow::{anyhow, Result};
use config::AppConfig;
use enclave_node::{listen_for_shutdown, setup_ciphernode};
use tracing::info;

use crate::owo;

pub async fn execute(config: AppConfig) -> Result<()> {
    owo();

    // let address = Address::parse_checksummed(&config.address(), None).context("Invalid address")?;
    let Some(address) = config.address() else {
        return Err(anyhow!("You must provide an address"));
    };

    info!("LAUNCHING CIPHERNODE: ({})", address);

    let (bus, handle) = setup_ciphernode(config, address).await?;

    tokio::spawn(listen_for_shutdown(bus.into(), handle));

    std::future::pending::<()>().await;

    Ok(())
}
