use crate::owo;
use anyhow::{anyhow, Result};
use config::AppConfig;
use enclave_core::{listen_for_shutdown, start};
use tracing::{info, instrument};

#[instrument(name = "app", skip_all)]
pub async fn execute(config: AppConfig) -> Result<()> {
    owo();
    let Some(address) = config.address() else {
        return Err(anyhow!("You must provide an address"));
    };

    let (bus, handle, peer_id) = start::execute(config, address).await?;
    info!("LAUNCHING CIPHERNODE: ({}/{})", address, peer_id);

    tokio::spawn(listen_for_shutdown(bus.into(), handle));

    std::future::pending::<()>().await;

    // TODO: The following never runs after graceful shutdown but should this should be
    // investigated. Expect it has to do with Websocket connections not being correctly terminated
    // within alloy.
    info!("Process has shutdown!");

    Ok(())
}
