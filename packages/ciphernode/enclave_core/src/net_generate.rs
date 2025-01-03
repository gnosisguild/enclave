use actix::Actor;
use anyhow::{anyhow, Result};
use config::AppConfig;
use crypto::Cipher;
use events::{EnclaveEvent, EventBus, EventBusConfig, GetErrors};
use libp2p::{identity::Keypair, PeerId};
use net::NetRepositoryFactory;
use zeroize::Zeroize;

use crate::datastore::get_repositories;

pub async fn execute(config: &AppConfig) -> Result<PeerId> {
    let kp = Keypair::generate_ed25519();
    let peer_id = kp.public().to_peer_id();
    let mut bytes = kp.try_into_ed25519()?.to_bytes().to_vec();
    let cipher = Cipher::from_config(config).await?;
    let encrypted = cipher.encrypt_data(&mut bytes.clone())?;
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig {
        capture_history: true,
        deduplicate: true,
    })
    .start();
    let repositories = get_repositories(&config, &bus)?;
    bytes.zeroize();
    repositories.libp2p_keypair().write(&encrypted);
    if let Some(error) = bus.send(GetErrors::<EnclaveEvent>::new()).await?.first() {
        return Err(anyhow!(error.clone()));
    }

    Ok(peer_id)
}
