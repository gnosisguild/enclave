use anyhow::*;
use enclave_core::swarm_ps;

pub async fn execute() -> Result<()> {
    swarm_ps::execute().await?;
    Ok(())
}
