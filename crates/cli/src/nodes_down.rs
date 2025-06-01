use anyhow::*;
use e3_entrypoint::nodes::down;

pub async fn execute() -> Result<()> {
    down::execute().await?;
    Ok(())
}
