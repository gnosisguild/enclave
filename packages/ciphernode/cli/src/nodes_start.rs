use anyhow::*;
use e3_entrypoint::nodes::start;

pub async fn execute(id: &str) -> Result<()> {
    start::execute(id).await?;
    Ok(())
}
