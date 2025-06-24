use anyhow::*;

/// Purge all ciphernode data
pub async fn execute() -> Result<()> {
    e3_entrypoint::nodes::purge::execute().await?;
    Ok(())
}
