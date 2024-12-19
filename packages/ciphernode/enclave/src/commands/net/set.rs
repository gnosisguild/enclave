use actix::Actor;
use libp2p::identity::Keypair;
use alloy::primitives::hex;
use anyhow::{bail, Result};
use cipher::Cipher;
use config::AppConfig;
use dialoguer::{theme::ColorfulTheme, Password};
use enclave_core::{EventBus, GetErrors};
use enclave_node::get_repositories;

pub fn create_keypair(input: &String) -> Result<Keypair> {
    match hex::check(input) {
        Ok(()) => {
            match Keypair::ed25519_from_bytes(hex::decode(input)?) {
                Ok(kp) => Ok(kp),
                Err(e) => bail!("Invalid network keypair: {}", e),
            }
        }
        Err(e) => bail!("Error decoding network keypair: {}", e),
    }
}

fn validate_keypair_input(input: &String) -> Result<()> {
    create_keypair(input).map(|_| ())
}

pub async fn execute(config: &AppConfig, net_keypair: Option<String>) -> Result<()> {
    let input = if let Some(net_keypair) = net_keypair {
        let kp = create_keypair(&net_keypair)?;
        kp.try_into_ed25519()?.to_bytes().to_vec()
    } else {
        let kp = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter your network private key")
            .validate_with(validate_keypair_input)
            .interact()?
            .trim()
            .to_string();
        let kp = create_keypair(&kp)?;
        kp.try_into_ed25519()?.to_bytes().to_vec()
    };

    let cipher = Cipher::from_config(config).await?;
    let encrypted = cipher.encrypt_data(&mut input.clone())?;
    let bus = EventBus::new(true).start();
    let repositories = get_repositories(&config, &bus)?;

    // NOTE: We are writing an encrypted string here
    repositories.libp2p_keypair().write(&encrypted);

    let errors = bus.send(GetErrors).await?;
    if errors.len() > 0 {
        for error in errors.iter() {
            println!("{error}");
        }
        bail!("There were errors setting the network keypair")
    }

    println!("Network keypair has been successfully encrypted.");

    Ok(())
}
