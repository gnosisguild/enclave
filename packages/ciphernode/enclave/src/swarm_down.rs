use anyhow::*;
use enclave_core::swarm_down;

pub async fn execute() -> Result<()> {
    swarm_down::execute().await?;
    Ok(())
}
