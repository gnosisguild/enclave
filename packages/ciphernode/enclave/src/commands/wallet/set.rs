use actix::Actor;
use anyhow::{anyhow, bail, Result};
use cipher::Cipher;
use config::AppConfig;
use dialoguer::{theme::ColorfulTheme, Password};
use enclave_core::{EventBus, GetErrors};
use enclave_node::get_repositories;

pub fn validate_private_key(input: &String) -> Result<()> {
    // Require 0x prefix
    if !input.starts_with("0x") {
        return Err(anyhow!(
            "Invalid private key format: must start with '0x' prefix"
        ));
    }

    // Remove 0x prefix
    let key = &input[2..];

    // Check length
    if key.len() != 64 {
        return Err(anyhow!(
            "Invalid private key length: {}. Expected 64 characters after '0x' prefix",
            key.len()
        ));
    }

    // Validate hex characters and convert to bytes
    let bytes = (0..key.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&key[i..i + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|e| anyhow!("Invalid hex character: {}", e))?;

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
