use anyhow::{bail, Result};
use cipher::{FilePasswordManager, PasswordManager};
use config::AppConfig;
use zeroize::{Zeroize, Zeroizing};

use crate::helpers::prompt_password::prompt_password;

fn get_zeroizing_pw_vec(input: Option<String>) -> Result<Zeroizing<Vec<u8>>> {
    if let Some(mut pw_str) = input {
        if pw_str.trim().is_empty() {
            bail!("Password must not be blank")
        }
        let pw = Zeroizing::new(pw_str.trim().as_bytes().to_owned());
        pw_str.zeroize();
        return Ok(pw);
    }

    // First password entry
    let mut pw_str = prompt_password("Please enter a new password")?;
    if pw_str.trim().is_empty() {
        bail!("Password must not be blank")
    }

    // Second password entry for confirmation
    let mut confirm_pw_str = prompt_password("Please confirm your password")?;

    // Check if passwords match
    if pw_str.trim() != confirm_pw_str.trim() {
        // Clean up sensitive data
        pw_str.zeroize();
        confirm_pw_str.zeroize();
        bail!("Passwords do not match")
    }

    let pw = Zeroizing::new(pw_str.trim().as_bytes().to_owned());

    // Clean up sensitive data
    pw_str.zeroize();
    confirm_pw_str.zeroize();

    Ok(pw)
}

pub async fn execute(config: &AppConfig, input: Option<String>, overwrite: bool) -> Result<()> {
    let key_file = config.key_file();
    let mut pm = FilePasswordManager::new(key_file);

    if overwrite && pm.is_set() {
        pm.delete_key().await?;
    }

    if pm.is_set() {
        bail!("Keyfile already exists. Refusing to overwrite. Try using `enclave password overwrite` or `enclave password delete` in order to change or delete your password.")
    }

    let pw = get_zeroizing_pw_vec(input)?;

    match pm.set_key(pw).await {
        Ok(_) => println!("Password sucessfully set."),
        Err(err) => println!("{}", err),
    };

    Ok(())
}
