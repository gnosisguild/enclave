use anyhow::*;
use config::AppConfig;
use enclave_core::net;

pub async fn execute(config: &AppConfig) -> Result<()> {
    net::purge::execute(config).await?;
    println!("Peer ID has been purged. A new Peer ID will be generated upon restart.");
    Ok(())
}
