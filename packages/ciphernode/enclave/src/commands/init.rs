use anyhow::Result;
use config::load_config;
use dialoguer::{theme::ColorfulTheme, Input};
use enclave_core::get_tag;
use std::fs;
use tracing::instrument;

use crate::commands::password::{self, PasswordCommands};

#[instrument(name = "app", skip_all, fields(id = get_tag()))]
pub async fn execute() -> Result<()> {
    // Prompt for Ethereum address
    let eth_address: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter your Ethereum address")
        .validate_with(|input: &String| -> Result<(), &str> {
            // Basic Ethereum address validation
            if !input.starts_with("0x") {
                return Err("Address must start with '0x'");
            }
            if input.len() != 42 {
                return Err("Address must be 42 characters long (including '0x')");
            }
            if !input[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                return Err("Address must contain only hexadecimal characters");
            }
            Ok(())
        })
        .interact()?;

    // Create config directory if it doesn't exist
    let config_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".config")
        .join("enclave");
    fs::create_dir_all(&config_dir)?;

    // Create config file path
    let config_path = config_dir.join("config.yaml");

    // Create YAML content using indented heredoc style
    let config_content = format!(
        r#"---
# Enclave Configuration File
# Ethereum Account Configuration
address: "{}"
"#,
        eth_address
    );

    // Write to file
    fs::write(config_path.clone(), config_content)?;

    // Load with default location
    let config = load_config(Some(&config_path.display().to_string()))?;

    password::execute(PasswordCommands::Create { password: None }, config).await?;
    
    // password::execute(/* command */, config)
    println!("Enclave configuration successfully created!");
    println!("You can start your node using `enclave start`");

    Ok(())
}
