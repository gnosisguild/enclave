use actix::Actor;
use anyhow::Result;
use cipher::Cipher;
use config::AppConfig;
use enclave_core::{EventBus, GetErrors};
use enclave_node::get_repositories;

pub async fn execute(config: &AppConfig, input: String) -> Result<()> {
    let cipher = Cipher::from_config(config).await?;
    let mut vec_input = input.as_bytes().to_vec();
    let encrypted = cipher.encrypt_data(&mut vec_input)?;
    let bus = EventBus::new(true).start();
    let repositories = get_repositories(&config, &bus)?;
    repositories.eth_private_key().write(&encrypted);
    let errors = bus.send(GetErrors).await?;
    for error in errors.iter() {
        println!("{error}");
    }
    if errors.len() == 0 {
        println!("WalletKey key has been successfully encrypted.");
    }
    Ok(())
}
