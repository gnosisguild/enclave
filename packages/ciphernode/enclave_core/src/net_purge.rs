use crate::helpers::datastore::get_repositories;
use actix::Actor;
use anyhow::*;
use config::AppConfig;
use events::{EnclaveEvent, EventBus, EventBusConfig};
use net::NetRepositoryFactory;

pub async fn execute(config: &AppConfig) -> Result<()> {
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig {
        capture_history: true,
        deduplicate: true,
    })
    .start();
    let repositories = get_repositories(&config, &bus)?;
    repositories.libp2p_keypair().clear();
    Ok(())
}
