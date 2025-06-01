use anyhow::*;
use e3_entrypoint::nodes::status;

pub async fn execute(id: &str) -> Result<()> {
    status::execute(id).await?;
    Ok(())
}
