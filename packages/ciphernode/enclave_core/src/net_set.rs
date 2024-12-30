use actix::Actor;
use alloy::primitives::hex;
use anyhow::{anyhow, Result};
use cipher::Cipher;
use config::AppConfig;
use events::{EventBus, GetErrors};
use libp2p::identity::Keypair;
use net::NetRepositoryFactory;

use crate::datastore::get_repositories;

fn create_keypair(input: &String) -> Result<Keypair> {
    hex::check(&input)?;
    let kp = Keypair::ed25519_from_bytes(hex::decode(&input)?)?;
    Ok(kp)
}

pub fn validate_keypair_input(input: &String) -> Result<()> {
    create_keypair(input).map(|_| ())
}

pub async fn execute(config: &AppConfig, value: String) -> Result<()> {
    let kp = create_keypair(&value)?;
    let mut secret = kp.try_into_ed25519()?.to_bytes().to_vec();
    let cipher = Cipher::from_config(config).await?;
    let encrypted = cipher.encrypt_data(&mut secret)?;

    // TODO: Tighten this up by removing external use of bus as it is confusing
    let bus = EventBus::new(true).start();
    let repositories = get_repositories(&config, &bus)?;
    repositories.libp2p_keypair().write(&encrypted);

    if let Some(error) = bus.send(GetErrors).await?.first() {
        return Err(anyhow!(error.clone()));
    }

    Ok(())
}
