use crate::helpers::repository::get_static_repositories;
use anyhow::*;
use config::AppConfig;
use net::NetRepositoryFactory;

pub async fn execute(config: &AppConfig) -> Result<()> {
    let repositories = get_static_repositories(&config)?;
    repositories.libp2p_keypair().clear_sync().await?;
    Ok(())
}
