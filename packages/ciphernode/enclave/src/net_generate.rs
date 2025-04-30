use anyhow::Result;
use config::AppConfig;
use enclave_core::net;

pub async fn execute(config: &AppConfig) -> Result<()> {
    let peer_id = net::generate::execute(config).await?;
    println!("Generated new keypair with peer ID: {}", peer_id);
    println!("Network keypair has been successfully generated and encrypted.");
    Ok(())
}
