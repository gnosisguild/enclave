use alloy::primitives::hex;
use anyhow::Result;
use crypto::Cipher;
use e3_config::AppConfig;
use libp2p::identity::Keypair;
use net::NetRepositoryFactory;

use crate::helpers::datastore::get_repositories;

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
    let repositories = get_repositories(config)?;
    repositories.libp2p_keypair().write_sync(&encrypted).await?;
    Ok(())
}
