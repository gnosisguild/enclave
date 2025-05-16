use anyhow::{bail, Result};
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

pub async fn execute(config: &AppConfig, input: Option<String>) -> Result<()> {
    println!("Setting password...");
    enclave_core::password::set::preflight(config).await?;

    let pw = get_zeroizing_pw_vec(input)?;

    enclave_core::password::set::execute(config, pw).await?;

    println!("Password successfully set.");

    Ok(())
}
