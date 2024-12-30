use actix::Actor;
use anyhow::{anyhow, Result};
use cipher::Cipher;
use config::AppConfig;
use enclave_core::{EventBus, GetErrors};
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
    let bus = EventBus::new(true).start();
    let repositories = get_repositories(&config, &bus)?;
    bytes.zeroize();
    repositories.libp2p_keypair().write(&encrypted);
    if let Some(error) = bus.send(GetErrors).await?.first() {
        return Err(anyhow!(error.clone()));
    }

    Ok(peer_id)
}
