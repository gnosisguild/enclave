use anyhow::*;
use e3_config::AppConfig;
use e3_entrypoint::net;

pub async fn execute(config: &AppConfig) -> Result<()> {
    net::peer_id::purge::execute(config).await?;
    println!("Peer ID has been purged. A new Peer ID will be generated upon restart.");
    Ok(())
}
