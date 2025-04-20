use crate::helpers::swarm_client;
use anyhow::*;
use tracing::{error, instrument};

#[instrument(skip_all)]
pub async fn execute(id: &str) -> Result<()> {
    if !swarm_client::is_ready().await? {
        bail!("Swarm client is not ready. Did you forget tocall `enclave` swarm");
    }

    swarm_client::start(id).await?;

    Ok(())
}
