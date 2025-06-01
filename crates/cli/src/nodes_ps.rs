use anyhow::*;
use e3_entrypoint::nodes::ps;

pub async fn execute() -> Result<()> {
    ps::execute().await?;
    Ok(())
}
