// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::error::ZkError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tokio::fs;
use tracing::{debug, warn};

const VERSIONS_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/gnosisguild/enclave/main/crates/zk-prover/versions.json";

const BB_VERSION: &str = "3.0.0-nightly.20260102";
const CIRCUITS_VERSION: &str = "0.1.15";

/// Supported bb binary targets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BbTarget {
    Amd64Linux,
    Amd64Darwin,
    Arm64Linux,
    Arm64Darwin,
}

impl BbTarget {
    /// Detect the current system's target
    pub fn current() -> Option<Self> {
        match (std::env::consts::ARCH, std::env::consts::OS) {
            ("x86_64", "linux") => Some(Self::Amd64Linux),
            ("x86_64", "macos") => Some(Self::Amd64Darwin),
            ("aarch64", "linux") => Some(Self::Arm64Linux),
            ("aarch64", "macos") => Some(Self::Arm64Darwin),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Amd64Linux => "amd64-linux",
            Self::Amd64Darwin => "amd64-darwin",
            Self::Arm64Linux => "arm64-linux",
            Self::Arm64Darwin => "arm64-darwin",
        }
    }

    /// Returns (arch, os) for URL templating
    pub fn url_parts(&self) -> (&'static str, &'static str) {
        match self {
            Self::Amd64Linux => ("amd64", "linux"),
            Self::Amd64Darwin => ("amd64", "darwin"),
            Self::Arm64Linux => ("arm64", "linux"),
            Self::Arm64Darwin => ("arm64", "darwin"),
        }
    }
}

impl std::fmt::Display for BbTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkConfig {
    pub bb_download_url: String,
    #[serde(default)]
    pub bb_checksums: HashMap<String, String>,
    pub circuits_download_url: String,
    #[serde(default)]
    pub circuits_checksums: HashMap<String, String>,
    pub required_bb_version: String,
    pub required_circuits_version: String,
}

impl Default for ZkConfig {
    fn default() -> Self {
        Self {
            bb_download_url: "https://github.com/AztecProtocol/aztec-packages/releases/download/v{version}/barretenberg-{arch}-{os}.tar.gz".to_string(),
            circuits_download_url: "https://github.com/gnosisguild/enclave/releases/download/v{version}/circuits-{version}.tar.gz".to_string(),
            bb_checksums: HashMap::new(),
            circuits_checksums: HashMap::new(),
            required_bb_version: BB_VERSION.to_string(),
            required_circuits_version: CIRCUITS_VERSION.to_string(),
        }
    }
}

impl ZkConfig {
    pub async fn fetch_latest() -> Result<Self, ZkError> {
        let client = reqwest::Client::new();
        let response = client
            .get(VERSIONS_MANIFEST_URL)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| {
                ZkError::DownloadFailed(VERSIONS_MANIFEST_URL.to_string(), e.to_string())
            })?;

        if !response.status().is_success() {
            return Err(ZkError::DownloadFailed(
                VERSIONS_MANIFEST_URL.to_string(),
                format!("HTTP {}", response.status()),
            ));
        }

        let config: ZkConfig = response.json().await.map_err(|e| {
            ZkError::DownloadFailed(VERSIONS_MANIFEST_URL.to_string(), e.to_string())
        })?;

        Ok(config)
    }

    pub async fn fetch_or_default() -> Self {
        match Self::fetch_latest().await {
            Ok(config) => {
                debug!(
                    "fetched versions manifest: bb={}, circuits={}",
                    config.required_bb_version, config.required_circuits_version
                );
                config
            }
            Err(e) => {
                warn!("could not fetch versions manifest ({}), using defaults", e);
                Self::default()
            }
        }
    }

    pub async fn load(path: &Path) -> std::io::Result<Self> {
        let contents = fs::read_to_string(path).await?;
        serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Get checksum for a specific target from the remote manifest
    pub fn bb_checksum_for(&self, target: BbTarget) -> Option<&str> {
        self.bb_checksums.get(target.as_str()).map(|s| s.as_str())
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

    /// Verify downloaded bb binary against stored checksum
    pub fn verify_bb_checksum(&self, data: &[u8]) -> Result<(), ZkError> {
        verify_checksum("bb", data, self.bb_checksum.as_deref())
    }

    pub fn verify_circuit_checksum(&self, circuit_name: &str, data: &[u8]) -> Result<(), ZkError> {
        let expected = self.circuits.get(circuit_name).map(|c| c.checksum.as_str());
        verify_checksum(circuit_name, data, expected)
    }
}

pub fn verify_checksum(file: &str, data: &[u8], expected: Option<&str>) -> Result<(), ZkError> {
    let Some(expected) = expected else {
        debug!("no checksum provided for {}, skipping verification", file);
        return Ok(());
    };

    let mut hasher = Sha256::new();
    hasher.update(data);
    let actual = hex::encode(hasher.finalize());

    if actual != expected {
        return Err(ZkError::ChecksumMismatch {
            file: file.to_string(),
            expected: expected.to_string(),
            actual,
        });
    }

    debug!("checksum verified for {}", file);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::test_utils::get_tempdir;

    use super::*;

    // BbTarget tests
    #[test]
    fn test_bb_target_as_str() {
        assert_eq!(BbTarget::Amd64Linux.as_str(), "amd64-linux");
        assert_eq!(BbTarget::Amd64Darwin.as_str(), "amd64-darwin");
        assert_eq!(BbTarget::Arm64Linux.as_str(), "arm64-linux");
        assert_eq!(BbTarget::Arm64Darwin.as_str(), "arm64-darwin");
    }

    #[test]
    fn test_bb_target_url_parts() {
        assert_eq!(BbTarget::Amd64Linux.url_parts(), ("amd64", "linux"));
        assert_eq!(BbTarget::Amd64Darwin.url_parts(), ("amd64", "darwin"));
        assert_eq!(BbTarget::Arm64Linux.url_parts(), ("arm64", "linux"));
        assert_eq!(BbTarget::Arm64Darwin.url_parts(), ("arm64", "darwin"));
    }

    #[test]
    fn test_bb_target_current_returns_some_on_supported_platform() {
        let target = BbTarget::current();
        if let Some(t) = target {
            assert!(!t.as_str().is_empty());
        }
    }

    #[test]
    fn test_verify_checksum_success() {
        let data = b"hello world";
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";

        let result = verify_checksum("test-file", data, Some(expected));
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_checksum_mismatch() {
        let data = b"hello world";
        let wrong = "0000000000000000000000000000000000000000000000000000000000000000";

        let result = verify_checksum("test-file", data, Some(wrong));

        let Err(ZkError::ChecksumMismatch {
            file,
            expected,
            actual,
        }) = result
        else {
            panic!("expected ChecksumMismatch error");
        };
        assert_eq!(file, "test-file");
        assert_eq!(expected, wrong);
        assert_eq!(
            actual,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_verify_checksum_skipped_when_none() {
        let result = verify_checksum("test-file", b"any data", None);
        assert!(result.is_ok());
    }

    // VersionInfo tests (local installed state)

    #[test]
    fn test_version_info_verify_bb_checksum() {
        let info = VersionInfo {
            bb_version: Some("0.86.0".to_string()),
            bb_checksum: Some(
                "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".to_string(),
            ),
            ..Default::default()
        };

        assert!(info.verify_bb_checksum(b"hello world").is_ok());
    }

    #[test]
    fn test_version_info_verify_bb_checksum_mismatch() {
        let info = VersionInfo {
            bb_checksum: Some(
                "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            ),
            ..Default::default()
        };

        let result = info.verify_bb_checksum(b"hello world");
        assert!(matches!(result, Err(ZkError::ChecksumMismatch { .. })));
    }

    #[test]
    fn test_version_info_verify_bb_checksum_skipped_when_none() {
        let info = VersionInfo::default();
        assert!(info.verify_bb_checksum(b"any data").is_ok());
    }

    #[test]
    fn test_version_info_verify_circuit_checksum() {
        let mut circuits = HashMap::new();
        circuits.insert(
            "my-circuit".to_string(),
            CircuitInfo {
                file: "my-circuit.bin".to_string(),
                checksum: "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
                    .to_string(),
            },
        );

        let info = VersionInfo {
            circuits,
            ..Default::default()
        };

        assert!(info
            .verify_circuit_checksum("my-circuit", b"hello world")
            .is_ok());
        assert!(info
            .verify_circuit_checksum("unknown-circuit", b"hello world")
            .is_ok()); // skipped
    }

    #[test]
    fn test_version_info_serialization_roundtrip() {
        let info = VersionInfo {
            bb_version: Some("0.86.0".to_string()),
            bb_checksum: Some("abc123".to_string()),
            circuits_version: Some("0.1.0".to_string()),
            circuits: HashMap::new(),
            last_updated: Some("2026-01-27T10:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: VersionInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.bb_version, info.bb_version);
        assert_eq!(parsed.bb_checksum, info.bb_checksum);
        assert_eq!(parsed.circuits_version, info.circuits_version);
    }

    // ZkConfig tests (remote manifest with all targets)

    #[test]
    fn test_zk_config_bb_checksum_for_target() {
        let mut bb_checksums = HashMap::new();
        bb_checksums.insert("amd64-linux".to_string(), "checksum-amd64".to_string());
        bb_checksums.insert("arm64-darwin".to_string(), "checksum-arm64".to_string());

        let config = ZkConfig {
            bb_checksums,
            ..Default::default()
        };

        assert_eq!(
            config.bb_checksum_for(BbTarget::Amd64Linux),
            Some("checksum-amd64")
        );
        assert_eq!(
            config.bb_checksum_for(BbTarget::Arm64Darwin),
            Some("checksum-arm64")
        );
        assert_eq!(config.bb_checksum_for(BbTarget::Arm64Linux), None);
    }

    #[test]
    fn test_zk_config_default() {
        let config = ZkConfig::default();

        assert!(config.bb_download_url.contains("{version}"));
        assert!(config.bb_checksums.is_empty());
        assert_eq!(config.required_bb_version, BB_VERSION);
    }

    /// Integration test that downloads a real bb binary and verifies checksum.
    #[tokio::test]
    async fn test_download_and_verify_bb() {
        let Some(target) = BbTarget::current() else {
            println!("skipping test: unsupported platform");
            return;
        };

        // Known good checksums for bb v0.82.2
        // Update these when bumping BB_VERSION
        let checksums: HashMap<&str, &str> = [
            (
                "amd64-linux",
                "9740013d1aa0eb1b0bb2d71484c8b3debc5050a409bd5f12f8454fbfc7cb5419",
            ),
            (
                "amd64-darwin",
                "7874494dd1238655993a44b85d94e9dcc3589d29980eff8b03a7f167a45c32e4",
            ),
            (
                "arm64-linux",
                "ae6bf8518998523b4e135cd638f305a802f52e8dfa5ea9b1c210de7d04c55343",
            ),
            (
                "arm64-darwin",
                "6d353c05dbecc573d1b0ca992c8b222db8e873853b7910b792915629347f6789",
            ),
        ]
        .into_iter()
        .collect();

        let version = BB_VERSION;
        let (arch, os) = target.url_parts();
        let url = format!(
            "https://github.com/AztecProtocol/aztec-packages/releases/download/v{version}/barretenberg-{arch}-{os}.tar.gz"
        );

        println!("downloading {} from {}", target, url);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .timeout(Duration::from_secs(120))
            .send()
            .await
            .expect("failed to send request");

        assert!(
            response.status().is_success(),
            "download failed: {}",
            response.status()
        );

        let bytes = response
            .bytes()
            .await
            .expect("failed to read response body");
        println!("downloaded {} bytes", bytes.len());

        // Verify checksum
        let expected = checksums
            .get(target.as_str())
            .expect("no checksum for target");
        let result = verify_checksum(&format!("bb-{}", target), &bytes, Some(expected));

        assert!(result.is_ok(), "checksum verification failed: {:?}", result);
        println!("checksum verified for {}", target);

        // Test saving and loading through VersionInfo
        let temp = get_tempdir().expect("failed to create temp dir");
        let tarball_path = temp.path().join("bb.tar.gz");

        fs::write(&tarball_path, &bytes)
            .await
            .expect("failed to write tarball");
        assert!(tarball_path.exists());

        // Verify VersionInfo checksum method works
        let info = VersionInfo {
            bb_version: Some(version.to_string()),
            bb_checksum: Some(expected.to_string()),
            ..Default::default()
        };
        assert!(info.verify_bb_checksum(&bytes).is_ok());

        // Cleanup happens automatically when temp goes out of scope
        println!("test passed, temp dir cleaned up");
    }

    /// Test that checksum verification fails for corrupted data
    #[tokio::test]
    async fn test_download_checksum_mismatch_on_corruption() {
        let Some(target) = BbTarget::current() else {
            println!("skipping test: unsupported platform");
            return;
        };

        let version = BB_VERSION;
        let (arch, os) = target.url_parts();
        let url = format!(
            "https://github.com/AztecProtocol/aztec-packages/releases/download/v{version}/barretenberg-{arch}-{os}.tar.gz"
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .timeout(Duration::from_secs(120))
            .send()
            .await
            .expect("failed to send request");

        let mut bytes = response
            .bytes()
            .await
            .expect("failed to read body")
            .to_vec();

        // Corrupt the data
        if !bytes.is_empty() {
            bytes[0] ^= 0xFF;
        }

        // Use a valid checksum that won't match corrupted data
        let info = VersionInfo {
            bb_checksum: Some(
                "a56257d8edc226180f5a7093393e4adc99447368a65099bb34292bd261408b99".to_string(),
            ),
            ..Default::default()
        };

        let result = info.verify_bb_checksum(&bytes);
        assert!(matches!(result, Err(ZkError::ChecksumMismatch { .. })));
        println!("correctly detected corrupted download");
    }
}
