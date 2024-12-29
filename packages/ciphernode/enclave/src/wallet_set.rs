use actix::Actor;
use alloy::{hex::FromHex, primitives::FixedBytes, signers::local::PrivateKeySigner};
use anyhow::{anyhow, bail, Result};
use cipher::Cipher;
use config::AppConfig;
use dialoguer::{theme::ColorfulTheme, Password};
use enclave_core::{EventBus, GetErrors};
use enclave_node::get_repositories;
use evm::EthPrivateKeyRepositoryFactory;

pub fn validate_private_key(input: &String) -> Result<()> {
    let bytes =
        FixedBytes::<32>::from_hex(input).map_err(|e| anyhow!("Invalid private key: {}", e))?;
    let _ =
        PrivateKeySigner::from_bytes(&bytes).map_err(|e| anyhow!("Invalid private key: {}", e))?;
    Ok(())
}

pub async fn execute(config: &AppConfig, private_key: Option<String>) -> Result<()> {
    let input = if let Some(private_key) = private_key {
        validate_private_key(&private_key)?;
        private_key
    } else {
        Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter your Ethereum private key")
            .validate_with(validate_private_key)
            .interact()?
            .trim()
            .to_string()
    };

    let cipher = Cipher::from_config(config).await?;
    let encrypted = cipher.encrypt_data(&mut input.as_bytes().to_vec())?;
    let bus = EventBus::new(true).start();
    let repositories = get_repositories(&config, &bus)?;

    // NOTE: We are writing an encrypted string here
    repositories.eth_private_key().write(&encrypted);

    let errors = bus.send(GetErrors).await?;
    if errors.len() > 0 {
        for error in errors.iter() {
            println!("{error}");
        }
        bail!("There were errors setting the wallet key")
    }

    println!("WalletKey key has been successfully encrypted.");

    Ok(())
}
