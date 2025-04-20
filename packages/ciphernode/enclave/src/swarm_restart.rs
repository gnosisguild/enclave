use anyhow::*;
use enclave_core::swarm_restart;

pub async fn execute(id: &str) -> Result<()> {
    swarm_restart::execute(id).await?;
    Ok(())
}
