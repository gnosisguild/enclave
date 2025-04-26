use alloy::{hex::FromHex, primitives::FixedBytes, signers::local::PrivateKeySigner};
use anyhow::{anyhow, Result};
use config::AppConfig;
use crypto::Cipher;
use evm::EthPrivateKeyRepositoryFactory;

use crate::helpers::{datastore::get_repositories, rand::generate_random_bytes};

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
    let repositories = get_repositories(config)?;
    repositories
        .eth_private_key()
        .write_sync(&encrypted)
        .await?;
    Ok(())
}

pub async fn autowallet(config: &AppConfig) -> Result<()> {
    let bytes = generate_random_bytes(32);
    let input = hex::encode(&bytes);
    execute(config, input).await?;
    Ok(())
}
