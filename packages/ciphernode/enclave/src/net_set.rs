use anyhow::Result;
use config::AppConfig;
use dialoguer::{theme::ColorfulTheme, Password};
use enclave_core::net::{self, set::validate_keypair_input};

pub async fn execute(config: &AppConfig, net_keypair: Option<String>) -> Result<()> {
    let input = if let Some(nkp) = net_keypair {
        nkp
    } else {
        Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter your network private key")
            .validate_with(validate_keypair_input)
            .interact()?
            .trim()
            .to_string()
    };

    net::set::execute(config, input).await?;

    println!("Network keypair has been successfully stored and encrypted.");

    Ok(())
}
