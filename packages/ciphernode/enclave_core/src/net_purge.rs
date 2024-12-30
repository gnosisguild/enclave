use crate::datastore::get_repositories;
use actix::Actor;
use anyhow::*;
use config::AppConfig;
use events::EventBus;
use net::NetRepositoryFactory;

pub async fn execute(config: &AppConfig) -> Result<()> {
    let bus = EventBus::new(true).start();
    let repositories = get_repositories(&config, &bus)?;
    repositories.libp2p_keypair().clear();
    Ok(())
}
