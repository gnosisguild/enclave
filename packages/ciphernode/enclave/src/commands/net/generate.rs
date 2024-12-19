use actix::Actor;
use anyhow::{bail, Result};
use cipher::Cipher;
use config::AppConfig;
use enclave_core::{EventBus, GetErrors};
use enclave_node::get_repositories;
use libp2p::identity::Keypair;

pub async fn execute(config: &AppConfig) -> Result<()> {
    let kp = Keypair::generate_ed25519();
    println!(
        "Generated new keypair with peer ID: {}",
        kp.public().to_peer_id()
    );
    let bytes = kp.try_into_ed25519()?.to_bytes().to_vec();
    let cipher = Cipher::from_config(config).await?;
    let encrypted = cipher.encrypt_data(&mut bytes.clone())?;
    let bus = EventBus::new(true).start();
    let repositories = get_repositories(&config, &bus)?;

    // NOTE: We are writing an encrypted string here
    repositories.libp2p_keypair().write(&encrypted);

    let errors = bus.send(GetErrors).await?;
    if errors.len() > 0 {
        for error in errors.iter() {
            println!("{error}");
        }
        bail!("There were errors generating the network keypair")
    }

    println!("Network keypair has been successfully generated and encrypted.");

    Ok(())
}
