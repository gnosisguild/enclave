use anyhow::{bail, Result};
use config::AppConfig;
use crypto::{FilePasswordManager, PasswordManager};
use zeroize::Zeroizing;

use crate::helpers::rand::generate_random_bytes;

pub async fn preflight(config: &AppConfig, overwrite: bool) -> Result<()> {
    let key_file = config.key_file();
    let pm = FilePasswordManager::new(key_file);

    if !overwrite && pm.is_set() {
        bail!("Keyfile already exists. Refusing to overwrite. Try using `enclave password overwrite` or `enclave password delete` in order to change or delete your password.")
    }

    Ok(())
}

pub async fn execute(config: &AppConfig, pw: Zeroizing<Vec<u8>>, overwrite: bool) -> Result<()> {
    let key_file = config.key_file();
    let mut pm = FilePasswordManager::new(key_file);

    if overwrite && pm.is_set() {
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
        execute(config, pw.into(), false).await?;
    }
    Ok(())
}
