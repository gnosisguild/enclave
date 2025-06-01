use anyhow::*;
use enclave_core::nodes::status;

pub async fn execute(id: &str) -> Result<()> {
    status::execute(id).await?;
    Ok(())
}
