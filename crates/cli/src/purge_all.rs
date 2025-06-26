use anyhow::Result;

/// Purge all local data anc cache
pub async fn execute() -> Result<()> {
    e3_entrypoint::nodes::purge::execute().await?;
    e3_support_scripts::program_cache_purge().await?;
    Ok(())
}
