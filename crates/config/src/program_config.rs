// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Program execution configuration (RISC Zero / Boundless).
//!
//! Extracted from [`AppConfig`] — these types configure external program
//! execution, not the ciphernode itself.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct BoundlessConfig {
    pub rpc_url: String,
    pub private_key: String,
    #[serde(default)]
    pub pinata_jwt: Option<String>,
    #[serde(default)]
    pub program_url: Option<String>,
    #[serde(default = "default_true")]
    pub onchain: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Risc0Config {
    #[serde(default)]
    pub risc0_dev_mode: u8,
    #[serde(default)]
    pub boundless: Option<BoundlessConfig>,
}

impl Default for Risc0Config {
    fn default() -> Self {
        Risc0Config {
            risc0_dev_mode: 1,
            boundless: None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ProgramConfig {
    risc0: Option<Risc0Config>,
    dev: Option<bool>,
}

impl ProgramConfig {
    pub fn risc0(&self) -> Option<&Risc0Config> {
        self.risc0.as_ref()
    }

    pub fn dev(&self) -> bool {
        self.dev.unwrap_or(false)
    }
}
