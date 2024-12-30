use anyhow::{bail, Result};
use config::AppConfig;
use crypto::{FilePasswordManager, PasswordManager};
use zeroize::Zeroizing;

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
