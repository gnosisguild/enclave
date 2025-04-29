use anyhow::Result;
use config::AppConfig;
use dialoguer::{theme::ColorfulTheme, Password};
use enclave_core::wallet::set::validate_private_key;

pub async fn execute(config: &AppConfig, private_key: Option<String>) -> Result<()> {
    let input = if let Some(private_key) = private_key {
        validate_private_key(&private_key)?;
        private_key
    } else {
        Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter your Ethereum private key")
            .validate_with(validate_private_key)
            .interact()?
            .trim()
            .to_string()
    };

    enclave_core::wallet::set::execute(config, input).await?;

    println!("WalletKey key has been successfully stored and encrypted.");

    Ok(())
}
