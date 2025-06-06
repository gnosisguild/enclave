use anyhow::*;
use tracing::{error, instrument};

use super::client;

#[instrument(skip_all)]
pub async fn execute(id: &str) -> Result<()> {
    if !client::is_ready().await? {
        bail!("Swarm client is not ready. Did you forget to call `enclave nodes up`?");
    }

    client::start(id).await?;

    Ok(())
}
