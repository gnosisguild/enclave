use anyhow::*;
use tracing::instrument;

use super::client;

#[instrument(skip_all)]
pub async fn execute() -> Result<()> {
    client::ps().await?;

    Ok(())
}
