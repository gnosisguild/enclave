use anyhow::Result;
use crypto::Cipher;
use e3_config::AppConfig;
use libp2p::{identity::Keypair, PeerId};
use net::NetRepositoryFactory;
use zeroize::Zeroize;

use crate::helpers::datastore::get_repositories;

pub async fn execute(config: &AppConfig) -> Result<PeerId> {
    let kp = Keypair::generate_ed25519();
    let peer_id = kp.public().to_peer_id();
    let mut bytes = kp.try_into_ed25519()?.to_bytes().to_vec();
    let cipher = Cipher::from_config(config).await?;
    let encrypted = cipher.encrypt_data(&mut bytes.clone())?;
    let repositories = get_repositories(config)?;
    bytes.zeroize();
    repositories.libp2p_keypair().write_sync(&encrypted).await?;

    Ok(peer_id)
}

pub async fn autonetkey(config: &AppConfig) -> Result<()> {
    let repositories = get_repositories(config)?;
    if !repositories.libp2p_keypair().has().await {
        execute(config).await?;
    }
    Ok(())
}
