// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Password};

pub fn prompt_password(prompt: impl Into<String>) -> Result<String> {
    let password = Password::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact()?;

    Ok(password)
}
