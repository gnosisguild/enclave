use actix::Actor;
use anyhow::*;
use config::AppConfig;
use enclave_core::EventBus;
use enclave_node::get_repositories;

pub async fn execute(config: &AppConfig) -> Result<()> {
    let bus = EventBus::new(true).start();
    let repositories = get_repositories(&config, &bus)?;
    repositories.libp2pid().clear();
    println!("Peer ID has been purged. A new peer will be generated upon restart.");
    Ok(())
}
