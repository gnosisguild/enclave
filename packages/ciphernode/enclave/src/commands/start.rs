use anyhow::{anyhow, Result};
use config::AppConfig;
use enclave_node::{listen_for_shutdown, setup_ciphernode};
use tracing::info;

use crate::owo;

pub async fn execute(config: AppConfig, id: &str) -> Result<()> {
    owo();

    // let address = Address::parse_checksummed(&config.address(), None).context("Invalid address")?;
    let Some(address) = config.address() else {
        return Err(anyhow!("You must provide an address"));
    };

    let (bus, handle, peer_id) = setup_ciphernode(config, address, id).await?;
    info!("LAUNCHING CIPHERNODE: ({}/{})", address, peer_id);

    tokio::spawn(listen_for_shutdown(bus.into(), handle));

    std::future::pending::<()>().await;

    Ok(())
}
