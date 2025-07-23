// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Password};
use e3_config::AppConfig;
use e3_entrypoint::net::{self, keypair::set::validate_keypair_input};

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

    net::keypair::set::execute(config, input).await?;

    println!("Network keypair has been successfully stored and encrypted.");

    Ok(())
}
