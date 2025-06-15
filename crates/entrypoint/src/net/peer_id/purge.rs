use crate::helpers::datastore::get_repositories;
use anyhow::*;
use e3_config::AppConfig;
use e3_net::NetRepositoryFactory;

pub async fn execute(config: &AppConfig) -> Result<()> {
    let repositories = get_repositories(config)?;
    repositories.libp2p_keypair().clear();
    Ok(())
}
