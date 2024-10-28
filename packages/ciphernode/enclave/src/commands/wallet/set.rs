use actix::Actor;
use anyhow::{bail, Result};
use cipher::Cipher;
use config::AppConfig;
use enclave_core::{EventBus, GetErrors};
use enclave_node::get_repositories;

pub async fn execute(config: &AppConfig, input: String) -> Result<()> {
    println!("WALLET KEY: {}", input);
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
