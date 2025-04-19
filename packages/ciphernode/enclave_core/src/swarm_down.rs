use crate::helpers::swarm_client;
use anyhow::*;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn execute() -> Result<()> {
    if !swarm_client::is_ready().await? {
        // not running!
        return Ok(());
    }

    swarm_client::terminate().await?;

    Ok(())
}
