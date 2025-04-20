use crate::helpers::swarm_client;
use anyhow::*;
use tracing::{error, instrument};

#[instrument(skip_all)]
pub async fn execute() -> Result<()> {
    swarm_client::ps().await?;

    Ok(())
}
