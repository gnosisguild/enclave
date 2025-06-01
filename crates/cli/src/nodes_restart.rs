use anyhow::*;
use e3_entrypoint::nodes::restart;

pub async fn execute(id: &str) -> Result<()> {
    restart::execute(id).await?;
    Ok(())
}
