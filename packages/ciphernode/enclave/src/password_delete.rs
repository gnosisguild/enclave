use crate::helpers::prompt_password::prompt_password;
use anyhow::Result;
use config::AppConfig;
use dialoguer::{theme::ColorfulTheme, Confirm};
use runtime::password_delete;
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

    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Are you sure you want to {mode} the key? This action cannot be undone."
        ))
        .default(false)
        .interact()?
    {
        return Ok(false);
    }

    let Ok(mut cur_pw) = password_delete::get_current_password(config).await else {
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
    if prompt_delete(config, DeleteMode::Delete).await? {
        password_delete::execute(config).await?;
        println!("Key successfully deleted.");
    } else {
        println!("Operation cancelled.");
    }
    Ok(())
}
