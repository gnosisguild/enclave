use actix::Actor;
use anyhow::*;
use config::AppConfig;
use enclave_core::EventBus;
use enclave_node::get_repositories;
use net::NetRepositoryFactory;

pub async fn execute(config: &AppConfig) -> Result<()> {
    let bus = EventBus::new(true).start();
    let repositories = get_repositories(&config, &bus)?;
    repositories.libp2p_keypair().clear();
    println!("Peer ID has been purged. A new Peer ID will be generated upon restart.");
    Ok(())
}
