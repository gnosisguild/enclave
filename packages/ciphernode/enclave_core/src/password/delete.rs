use anyhow::*;
use e3_config::AppConfig;
use e3_crypto::{FilePasswordManager, PasswordManager};
use zeroize::Zeroizing;

pub async fn get_current_password(config: &AppConfig) -> Result<Zeroizing<String>> {
    let key_file = config.key_file();
    let pm = FilePasswordManager::new(key_file);
    if !pm.is_set() {
        bail!("Password is not set. Nothing to do.")
    }
    let pw = pm.get_key().await?;
    Ok(Zeroizing::new(String::from_utf8_lossy(&pw).to_string()))
}

pub async fn execute(config: &AppConfig) -> Result<()> {
    let key_file = config.key_file();
    let mut pm = FilePasswordManager::new(key_file);
    pm.delete_key().await?;
    Ok(())
}
