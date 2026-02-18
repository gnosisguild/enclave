// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Password};
use e3_config::AppConfig;
use e3_entrypoint::wallet::set::validate_private_key;
use e3_utils::eth_address_from_private_key;
use zeroize::Zeroizing;

pub fn ask_for_private_key(given_key: Option<Zeroizing<String>>) -> Result<Zeroizing<String>> {
    let key = if let Some(given_key) = given_key {
        validate_private_key(&given_key)?;
        given_key
    } else {
        Zeroizing::new(
            Password::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter your Ethereum private key")
                .validate_with(validate_private_key)
                .interact()?
                .trim()
                .to_string(),
        )
    };

    Ok(key)
}

pub async fn execute(config: &AppConfig, private_key: Option<Zeroizing<String>>) -> Result<()> {
    let input = ask_for_private_key(private_key)?;
    let address = eth_address_from_private_key(&input)?;
    e3_entrypoint::wallet::set::execute(config, input).await?;
    e3_entrypoint::config::set_address::execute(config, address)?;
    println!("Wallet key has been successfully stored and encrypted.");

    Ok(())
}
