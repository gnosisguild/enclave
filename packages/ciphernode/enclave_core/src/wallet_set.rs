use crate::helpers::{crypto::generate_random_bytes, repository::get_static_repositories};
use alloy::{hex::FromHex, primitives::FixedBytes, signers::local::PrivateKeySigner};
use anyhow::{anyhow, Result};
use config::AppConfig;
use crypto::Cipher;
use evm::EthPrivateKeyRepositoryFactory;

pub fn validate_private_key(input: &String) -> Result<()> {
    let bytes =
        FixedBytes::<32>::from_hex(input).map_err(|e| anyhow!("Invalid private key: {}", e))?;
    let _ =
        PrivateKeySigner::from_bytes(&bytes).map_err(|e| anyhow!("Invalid private key: {}", e))?;
    Ok(())
}

pub async fn execute(config: &AppConfig, input: String) -> Result<()> {
    let cipher = Cipher::from_config(config).await?;
    let encrypted = cipher.encrypt_data(&mut input.as_bytes().to_vec())?;
    let repositories = get_static_repositories(config)?;
    repositories
        .eth_private_key()
        .write_sync(&encrypted)
        .await?;
    Ok(())
}

pub async fn autowallet(config: &AppConfig) -> Result<()> {
    let private_key = generate_random_bytes(32);
    let hex_string = format!("0x{}", hex::encode(private_key));
    let repositories = get_static_repositories(config)?;
    println!("autowallet... trying to retrieve from db");
    if !repositories.eth_private_key().has().await {
        println!("key does not exist!");
        execute(config, hex_string).await?
    }
    Ok(())
}
