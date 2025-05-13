use crate::helpers::datastore::get_repositories;
use anyhow::*;
use config::AppConfig;
use net::NetRepositoryFactory;

pub async fn execute(config: &AppConfig) -> Result<()> {
    let repositories = get_repositories(config)?;
    repositories.libp2p_keypair().clear();
    Ok(())
}
