use anyhow::*;
use enclave_core::nodes::down;

pub async fn execute() -> Result<()> {
    down::execute().await?;
    Ok(())
}
