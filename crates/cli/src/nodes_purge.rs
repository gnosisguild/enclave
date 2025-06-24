use anyhow::*;
use e3_entrypoint::nodes::purge;

/// Purge all ciphernode data
pub async fn execute() -> Result<()> {
    purge::execute().await?;
    Ok(())
}
