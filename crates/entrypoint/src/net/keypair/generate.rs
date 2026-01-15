// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_config::AppConfig;
use e3_crypto::Cipher;
use e3_net::NetRepositoryFactory;
use libp2p::{identity::Keypair, PeerId};
use tracing::warn;
use zeroize::Zeroize;

use crate::helpers::datastore::get_repositories;

pub async fn execute(config: &AppConfig) -> Result<PeerId> {
    let kp = Keypair::generate_ed25519();
    let peer_id = kp.public().to_peer_id();
    let mut bytes = kp.try_into_ed25519()?.to_bytes().to_vec();
    let cipher = Cipher::from_file(config.key_file()).await?;
    let encrypted = cipher.encrypt_data(&mut bytes.clone())?;
    let repositories = get_repositories(config)?;
    bytes.zeroize();
    repositories.libp2p_keypair().write_sync(&encrypted).await?;

    Ok(peer_id)
}

pub async fn autonetkey(config: &AppConfig) -> Result<()> {
    let repositories = get_repositories(config)?;
    if !repositories.libp2p_keypair().has().await {
        warn!("Auto-generating network keypair because 'autonetkey: true' is set and no keypair exists.");
        warn!("This will create a NEW peer identity. If your data directory is not persistent");
        warn!("(e.g., running in Docker without volumes), a new identity will be generated on each restart,");
        warn!("which will cause network connectivity issues with other peers.");
        warn!("For production use, run 'enclave net keypair (generate)/(set --net-keypair <YOUR_PEER_ID>)' once and ensure data persistence.");
        execute(config).await?;
    }
    Ok(())
}
