// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

pub const REQUIRED_BB_VERSION: &str = "0.86.0";
pub const REQUIRED_CIRCUITS_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoirConfig {
    pub bb_download_url: String,
    pub circuits_download_url: String,
    pub required_bb_version: String,
    pub required_circuits_version: String,
}

impl Default for NoirConfig {
    fn default() -> Self {
        Self {
            bb_download_url: "https://github.com/AztecProtocol/aztec-packages/releases/download/v{version}/barretenberg-{arch}-{os}.tar.gz".to_string(),
            circuits_download_url: "https://github.com/gnosisguild/enclave/releases/download/v{version}/circuits.tar.gz".to_string(),
            required_bb_version: REQUIRED_BB_VERSION.to_string(),
            required_circuits_version: REQUIRED_CIRCUITS_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionInfo {
    #[serde(default)]
    pub bb_version: Option<String>,
    #[serde(default)]
    pub bb_checksum: Option<String>,
    #[serde(default)]
    pub circuits_version: Option<String>,
    #[serde(default)]
    pub circuits: HashMap<String, CircuitInfo>,
    #[serde(default)]
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitInfo {
    pub file: String,
    pub checksum: String,
}

impl VersionInfo {
    pub async fn load(path: &Path) -> std::io::Result<Self> {
        let contents = fs::read_to_string(path).await?;
        serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub async fn save(&self, path: &Path) -> std::io::Result<()> {
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents).await
    }

    pub fn bb_matches(&self, required: &str) -> bool {
        self.bb_version.as_deref() == Some(required)
    }

    pub fn circuits_match(&self, required: &str) -> bool {
        self.circuits_version.as_deref() == Some(required)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info_serialization() {
        let info = VersionInfo {
            bb_version: Some("0.87.0".to_string()),
            bb_checksum: Some("abc123".to_string()),
            circuits_version: Some("0.1.0".to_string()),
            circuits: HashMap::new(),
            last_updated: Some("2026-01-27T10:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: VersionInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.bb_version, info.bb_version);
        assert_eq!(parsed.circuits_version, info.circuits_version);
    }
}
