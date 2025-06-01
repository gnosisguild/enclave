use crate::helpers::prompt_password::prompt_password;
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Confirm};
use e3_config::AppConfig;
use zeroize::Zeroize;

pub async fn prompt_delete(config: &AppConfig) -> Result<bool> {
    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Are you sure you want to delete the key? This action cannot be undone.")
        .default(false)
        .interact()?
    {
        return Ok(false);
    }

    let Ok(mut cur_pw) = enclave_core::password::delete::get_current_password(config).await else {
        println!("Password is not set. Nothing to do.");
        return Ok(false);
    };

    let mut pw_str = prompt_password("Please enter the current password")?;
    if pw_str != *cur_pw {
        // Clean up sensitive data
        pw_str.zeroize();
        cur_pw.zeroize();
        return Err(anyhow::anyhow!("Incorrect password"));
    }

    Ok(true)
}

pub async fn execute(config: &AppConfig) -> Result<()> {
    if prompt_delete(config).await? {
        enclave_core::password::delete::execute(config).await?;
        println!("Password successfully deleted.");
    } else {
        println!("Operation cancelled.");
    }
    Ok(())
}
