use anyhow::*;
use enclave_core::swarm_status;

pub async fn execute(id: &str) -> Result<()> {
    swarm_status::execute(id).await?;
    Ok(())
}
