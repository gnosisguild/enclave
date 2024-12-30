use crate::owo;
use anyhow::{anyhow, Result};
use config::AppConfig;
use enclave_core::{listen_for_shutdown, start};
use events::get_tag;
use tracing::{info, instrument};

#[instrument(name="app", skip_all,fields(id = get_tag()))]
pub async fn execute(config: AppConfig) -> Result<()> {
    owo();
    let Some(address) = config.address() else {
        return Err(anyhow!("You must provide an address"));
    };

    let (bus, handle, peer_id) = start::execute(config, address).await?;
    info!("LAUNCHING CIPHERNODE: ({}/{})", address, peer_id);

    tokio::spawn(listen_for_shutdown(bus.into(), handle));

    std::future::pending::<()>().await;

    Ok(())
}
