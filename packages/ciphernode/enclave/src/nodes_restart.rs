use anyhow::*;
use enclave_core::nodes::restart;

pub async fn execute(id: &str) -> Result<()> {
    restart::execute(id).await?;
    Ok(())
}
