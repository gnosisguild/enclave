use dialoguer::{theme::ColorfulTheme, Password};
use anyhow::Result;

pub fn prompt_password(prompt: impl Into<String>) -> Result<String> {
    let password = Password::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact()?;
    
    Ok(password)
}
