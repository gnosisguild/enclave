use anyhow::*;
use enclave_core::nodes::start;

pub async fn execute(id: &str) -> Result<()> {
    start::execute(id).await?;
    Ok(())
}
