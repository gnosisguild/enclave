use anyhow::*;
use config::AppConfig;
use enclave_core::{aggregator_start, listen_for_shutdown};
use events::get_tag;
use tracing::{info, instrument};

use crate::owo;

#[instrument(name="app", skip_all,fields(id = get_tag()))]
pub async fn execute(
    config: AppConfig,
    pubkey_write_path: Option<&str>,
    plaintext_write_path: Option<&str>,
) -> Result<()> {
    owo();

    let (bus, peer_id) =
        aggregator_start::execute(config, pubkey_write_path, plaintext_write_path).await?;

    info!("LAUNCHING AGGREGATOR {}", peer_id);
    tokio::spawn(listen_for_shutdown(bus.into()));

    std::future::pending::<()>().await;

    Ok(())
}
