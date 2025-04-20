use anyhow::*;
use enclave_core::swarm_stop;

pub async fn execute(id: &str) -> Result<()> {
    swarm_stop::execute(id).await?;
    Ok(())
}
