use anyhow::{bail, Result};
use crypto::{FilePasswordManager, PasswordManager};
use e3_config::AppConfig;
use zeroize::Zeroizing;

use crate::helpers::rand::generate_random_bytes;

pub async fn preflight(config: &AppConfig) -> Result<()> {
    let key_file = config.key_file();
    let pm = FilePasswordManager::new(key_file);

    if pm.is_set() {
        bail!("Keyfile already exists. Try using `enclave password set` to set a new password or `enclave password delete` to remove the existing one.")
    }

    Ok(())
}

pub async fn execute(config: &AppConfig, pw: Zeroizing<Vec<u8>>) -> Result<()> {
    let key_file = config.key_file();
    let mut pm = FilePasswordManager::new(key_file);

    // If a password exists, delete it first
    if pm.is_set() {
        pm.delete_key().await?;
    }

    pm.set_key(pw).await?;

    Ok(())
}

pub async fn autopassword(config: &AppConfig) -> Result<()> {
    let key_file = config.key_file();
    let pm = FilePasswordManager::new(key_file);
    if !pm.is_set() {
        let pw = generate_random_bytes(128);
        execute(config, pw.into()).await?;
    }
    Ok(())
}
