use anyhow::{bail, Result};
use config::AppConfig;
use crypto::{FilePasswordManager, PasswordManager};
use zeroize::Zeroizing;

use crate::helpers::crypto::generate_random_bytes;

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

pub async fn autopass(config: &AppConfig) -> Result<()> {
    let key_file = config.key_file();
    let mut pm = FilePasswordManager::new(key_file);
    if !pm.is_set() {
        let random_bytes = generate_random_bytes(16);
        let secure_bytes = Zeroizing::new(random_bytes);
        pm.set_key(secure_bytes).await?;
    }
    Ok(())
}
