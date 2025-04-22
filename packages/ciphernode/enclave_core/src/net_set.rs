use alloy::primitives::hex;
use anyhow::Result;
use config::AppConfig;
use crypto::Cipher;
use libp2p::identity::Keypair;
use net::NetRepositoryFactory;

use crate::helpers::{crypto::generate_random_bytes, repository::get_static_repositories};

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

    println!("before before");
    let repositories = get_static_repositories(&config)?;
    repositories.libp2p_keypair().write_sync(&encrypted).await?;

    println!("data written");
    Ok(())
}

pub async fn autonet(config: &AppConfig) -> Result<()> {
    let private_key = generate_random_bytes(32);
    let hex_string = format!("0x{}", hex::encode(private_key));
    let repositories = get_static_repositories(&config)?;
    println!("autonet trying to check for data...");
    if !repositories.libp2p_keypair().has().await {
        println!("data does not exist! writing...");
        execute(config, hex_string).await?
    }
    Ok(())
}
