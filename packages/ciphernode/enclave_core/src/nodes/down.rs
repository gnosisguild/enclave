use anyhow::*;
use tracing::instrument;

use super::client;

#[instrument(skip_all)]
pub async fn execute() -> Result<()> {
    if !client::is_ready().await? {
        // not running!
        return Ok(());
    }

    client::terminate().await?;

    Ok(())
}
