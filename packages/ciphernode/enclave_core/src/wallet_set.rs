use actix::Actor;
use alloy::{hex::FromHex, primitives::FixedBytes, signers::local::PrivateKeySigner};
use anyhow::{anyhow, Result};
use config::AppConfig;
use crypto::Cipher;
use events::{EnclaveEvent, EventBus, EventBusConfig, GetErrors};
use evm::EthPrivateKeyRepositoryFactory;

use crate::datastore::get_repositories;

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
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig {
        capture_history: true,
        deduplicate: true,
    })
    .start();
    let repositories = get_repositories(&config, &bus)?;
    repositories.eth_private_key().write(&encrypted);
    if let Some(error) = bus.send(GetErrors::<EnclaveEvent>::new()).await?.first() {
        return Err(anyhow!(error.clone()));
    }
    Ok(())
}
