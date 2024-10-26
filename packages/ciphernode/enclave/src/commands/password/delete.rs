use anyhow::*;
use cipher::{FilePasswordManager, PasswordManager};
use config::AppConfig;
use dialoguer::{theme::ColorfulTheme, Confirm};
use rpassword::prompt_password;
use zeroize::Zeroize;

pub enum DeleteMode {
    Delete,
    Overwrite,
}

impl DeleteMode {
    fn to_string(&self) -> String {
        match self {
            DeleteMode::Delete => "delete".to_owned(),
            DeleteMode::Overwrite => "overwrite".to_owned(),
        }
    }
}

pub async fn prompt_delete(config: &AppConfig, delete_mode: DeleteMode) -> Result<bool> {
    let mode = delete_mode.to_string();
    let proceed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Are you sure you want to {mode} the key? This action cannot be undone."
        ))
        .default(false)
        .interact()?;

    if proceed {
        let mut pw_str = prompt_password("\n\nPlease enter the current password: ")?;
        let key_file = config.key_file();
        let mut pm = FilePasswordManager::new(key_file);
        let mut cur_pw = pm.get_key().await?;

        if pw_str != String::from_utf8_lossy(&cur_pw) {
            // Clean up sensitive data
            pw_str.zeroize();
            cur_pw.zeroize();
            return Err(anyhow::anyhow!("Incorrect password"));
        }
        pm.delete_key().await?;
    } else {
        return Ok(false);
    }
    Ok(true)
}

pub async fn execute(config: &AppConfig) -> Result<()> {
    if prompt_delete(config, DeleteMode::Delete).await? {
        println!("Key successfully deleted.");
    } else {
        println!("Operation cancelled.");
    }
    Ok(())
}
