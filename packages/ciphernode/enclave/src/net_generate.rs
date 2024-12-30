use anyhow::Result;
use config::AppConfig;
use runtime::net_generate;

pub async fn execute(config: &AppConfig) -> Result<()> {
    let peer_id = net_generate::execute(config).await?;
    println!("Generated new keypair with peer ID: {}", peer_id);
    println!("Network keypair has been successfully generated and encrypted.");
    Ok(())
}
