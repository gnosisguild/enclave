use anyhow::Result;
use e3_config::AppConfig;
use e3_entrypoint::net;

pub async fn execute(config: &AppConfig) -> Result<()> {
    let peer_id = net::keypair::generate::execute(config).await?;
    println!("Generated new keypair with peer ID: {}", peer_id);
    println!("Network keypair has been successfully generated and encrypted.");
    Ok(())
}
