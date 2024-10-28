use anyhow::Result;
use cipher::{FilePasswordManager, PasswordManager};
use config::AppConfig;
use rpassword::prompt_password;
use zeroize::{Zeroize, Zeroizing};

fn get_zeroizing_pw_vec(input: Option<String>) -> Result<Zeroizing<Vec<u8>>> {
    if let Some(mut pw_str) = input {
        let pw = Zeroizing::new(pw_str.trim().as_bytes().to_owned());
        pw_str.zeroize();
        return Ok(pw);
    }

    // First password entry
    let mut pw_str = prompt_password("\n\nPlease enter a new password: ")?;
    // Second password entry for confirmation
    let mut confirm_pw_str = prompt_password("Please confirm your password: ")?;

    // Check if passwords match
    if pw_str.trim() != confirm_pw_str.trim() {
        // Clean up sensitive data
        pw_str.zeroize();
        confirm_pw_str.zeroize();
        return Err(anyhow::anyhow!("Passwords do not match"));
    }

    let pw = Zeroizing::new(pw_str.trim().as_bytes().to_owned());

    // Clean up sensitive data
    pw_str.zeroize();
    confirm_pw_str.zeroize();

    Ok(pw)
}

pub async fn execute(config: &AppConfig, input: Option<String>) -> Result<()> {
    let key_file = config.key_file();
    let mut pm = FilePasswordManager::new(key_file);
    let pw = get_zeroizing_pw_vec(input)?;

    match pm.set_key(pw).await {
        Ok(_) => println!("Password sucessfully set."),
        Err(err) => println!("{}", err),
    };

    Ok(())
}
