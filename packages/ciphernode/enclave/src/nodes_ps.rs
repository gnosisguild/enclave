use anyhow::*;
use enclave_core::nodes::ps;

pub async fn execute() -> Result<()> {
    ps::execute().await?;
    Ok(())
}
