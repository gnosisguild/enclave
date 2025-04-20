use anyhow::*;
use enclave_core::swarm_start;

pub async fn execute(id: &str) -> Result<()> {
    swarm_start::execute(id).await?;
    Ok(())
}
