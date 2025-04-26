use anyhow::*;
use enclave_core::nodes::stop;

pub async fn execute(id: &str) -> Result<()> {
    stop::execute(id).await?;
    Ok(())
}
