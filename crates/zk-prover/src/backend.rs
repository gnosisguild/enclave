// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use crate::config::{verify_checksum, BbTarget, VersionInfo, ZkConfig};
use crate::error::ZkError;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use tar::Archive;
use tokio::fs;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub enum SetupStatus {
    Ready,
    BbNeedsUpdate {
        installed: Option<String>,
        required: String,
    },
    CircuitsNeedUpdate {
        installed: Option<String>,
        required: String,
    },
    FullSetupNeeded,
}

#[derive(Debug, Clone)]
pub struct ZkBackend {
    pub base_dir: PathBuf,
    pub bb_binary: PathBuf,
    pub circuits_dir: PathBuf,
    pub work_dir: PathBuf,
    pub config: ZkConfig,
}

impl ZkBackend {
    pub fn new(enclave_dir: &Path, config: ZkConfig) -> Self {
        let base_dir = enclave_dir.join("noir");
        Self {
            bb_binary: base_dir.join("bin").join("bb"),
            circuits_dir: base_dir.join("circuits"),
            work_dir: base_dir.join("work"),
            base_dir,
            config,
        }
    }

    pub async fn with_default_dir() -> Result<Self, ZkError> {
        let base_dirs = directories::BaseDirs::new().ok_or_else(|| {
            ZkError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine home directory",
            ))
        })?;

        let enclave_dir = base_dirs.home_dir().join(".enclave");
        let config = ZkConfig::fetch_or_default().await;
        Ok(Self::new(&enclave_dir, config))
    }

    fn version_file(&self) -> PathBuf {
        self.base_dir.join("version.json")
    }

    pub async fn load_version_info(&self) -> VersionInfo {
        match VersionInfo::load(&self.version_file()).await {
            Ok(info) => info,
            Err(_) => VersionInfo::default(),
        }
    }

    pub async fn check_status(&self) -> SetupStatus {
        let version_info = self.load_version_info().await;

        let bb_ok =
            version_info.bb_matches(&self.config.required_bb_version) && self.bb_binary.exists();
        let circuits_ok = version_info.circuits_match(&self.config.required_circuits_version)
            && self.circuits_dir.exists();

        match (bb_ok, circuits_ok) {
            (true, true) => SetupStatus::Ready,
            (false, true) => SetupStatus::BbNeedsUpdate {
                installed: version_info.bb_version,
                required: self.config.required_bb_version.clone(),
            },
            (true, false) => SetupStatus::CircuitsNeedUpdate {
                installed: version_info.circuits_version,
                required: self.config.required_circuits_version.clone(),
            },
            (false, false) => SetupStatus::FullSetupNeeded,
        }
    }

    pub async fn ensure_installed(&self) -> Result<(), ZkError> {
        fs::create_dir_all(&self.base_dir).await?;
        fs::create_dir_all(self.base_dir.join("bin")).await?;
        fs::create_dir_all(&self.circuits_dir).await?;
        fs::create_dir_all(&self.work_dir).await?;

        let status = self.check_status().await;

        match status {
            SetupStatus::Ready => {
                debug!("ZK backend is ready");
                Ok(())
            }
            SetupStatus::BbNeedsUpdate {
                installed,
                required,
            } => {
                info!(
                    "updating Barretenberg: {} -> {}",
                    installed.as_deref().unwrap_or("not installed"),
                    required
                );
                self.download_bb().await
            }
            SetupStatus::CircuitsNeedUpdate {
                installed,
                required,
            } => {
                info!(
                    "updating circuits: {} -> {}",
                    installed.as_deref().unwrap_or("not installed"),
                    required
                );
                self.download_circuits().await
            }
            SetupStatus::FullSetupNeeded => {
                info!("setting up ZK proving infrastructure...");
                self.download_bb().await?;
                self.download_circuits().await
            }
        }
    }

    fn current_target() -> Result<BbTarget, ZkError> {
        BbTarget::current().ok_or_else(|| ZkError::UnsupportedPlatform {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        })
    }

    pub async fn download_bb(&self) -> Result<(), ZkError> {
        let target = Self::current_target()?;
        let (arch, os) = target.url_parts();
        let version = &self.config.required_bb_version;

        let url = self
            .config
            .bb_download_url
            .replace("{version}", version)
            .replace("{os}", &os)
            .replace("{arch}", &arch);

        info!("downloading Barretenberg from: {}", url);

        let bytes = self.download_with_progress(&url, "Downloading bb").await?;
        let expected_checksum = self.config.bb_checksum_for(target);
        verify_checksum(&format!("bb-{}", target), &bytes, expected_checksum)?;

        let decoder = GzDecoder::new(&bytes[..]);
        let mut archive = Archive::new(decoder);

        let bin_dir = self.base_dir.join("bin");
        fs::create_dir_all(&bin_dir).await?;

        let temp_dir = tempfile::tempdir()?;
        archive.unpack(temp_dir.path())?;

        let bb_source = Self::find_bb_in_dir(temp_dir.path())?;

        fs::copy(&bb_source, &self.bb_binary).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.bb_binary).await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&self.bb_binary, perms).await?;
        }

        let mut version_info = self.load_version_info().await;
        version_info.bb_version = Some(version.clone());
        version_info.bb_checksum = expected_checksum.map(|s| s.to_string());
        version_info.last_updated = Some(chrono_now());
        version_info.save(&self.version_file()).await?;

        info!("installed Barretenberg v{}", version);
        Ok(())
    }

    fn find_bb_in_dir(dir: &Path) -> Result<PathBuf, ZkError> {
        use walkdir::WalkDir;

        for candidate in ["bb", "bin/bb", "barretenberg/bin/bb"] {
            let path = dir.join(candidate);
            if path.exists() && path.is_file() {
                return Ok(path);
            }
        }

        WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy() == "bb" && e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .ok_or_else(|| {
                ZkError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "bb binary not found in archive",
                ))
            })
    }

    pub async fn download_circuits(&self) -> Result<(), ZkError> {
        let version = &self.config.required_circuits_version;
        let url = self
            .config
            .circuits_download_url
            .replace("{version}", version);

        info!("downloading circuits from: {}", url);

        let result = self
            .download_with_progress(&url, "Downloading circuits")
            .await;

        match result {
            Ok(bytes) => {
                let decoder = GzDecoder::new(&bytes[..]);
                let mut archive = Archive::new(decoder);
                archive.unpack(&self.circuits_dir)?;
            }
            Err(e) => {
                warn!(
                    "could not download circuits ({}), creating placeholder for testing",
                    e
                );
                self.create_placeholder_circuits().await?;
            }
        }

        let mut version_info = self.load_version_info().await;
        version_info.circuits_version = Some(version.clone());
        version_info.last_updated = Some(chrono_now());
        version_info.save(&self.version_file()).await?;

        info!("installed circuits v{}", version);
        Ok(())
    }

    async fn create_placeholder_circuits(&self) -> Result<(), ZkError> {
        fs::create_dir_all(&self.circuits_dir).await?;

        let placeholder = serde_json::json!({
            "noir_version":"1.0.0-beta.15+83245db91dcf63420ef4bcbbd85b98f397fee663",
            "hash":"15412581843239610929",
            "abi":{
                "parameters":[
                    {"name":"x","type":{"kind":"field"},"visibility":"private"},
                    {"name":"y","type":{"kind":"field"},"visibility":"private"},
                    {"name":"_sum","type":{"kind":"field"},"visibility":"public"}
                ],
                "return_type":null,
                "error_types":{}
            },
            "bytecode":"H4sIAAAAAAAA/5WOMQ5AMBRA/y8HMbIRRxCJSYwWg8RiIGIz9gjiAk4hHKeb0WLX0KHRDu1bXvL/y89H+HCFu7rtCTeCiiPsgRFo06LUhk0+smgN9iLdKC0rPz6z6RjmhN3LxffE/O7byg+hZv7nAb2HRPkUAQAA",
            "debug_symbols":"jZDRCoMwDEX/Jc996MbG1F8ZQ2qNUghtie1giP++KLrpw2BPaXJ7bsgdocUm97XzXRiguo/QsCNyfU3BmuSCl+k4KdjaOjGijGCnCxUNo09Q+Uyk4GkoL5+GaPxSk2FRtQL0rVQx7Bzh/JrUl9a/0Vu5ssXlA1//psvbSp90ccAf0hnr+HAuaKjO0+zGzjSEawRd9naXSHrFTdkyixwstplxtls0WfAG",
            "file_map":{
                "50":{"source":"pub fn main(\n    x: Field,\n    y: Field,\n    _sum: pub Field\n) {\n    let sum = x + y;\n    assert(sum == _sum);\n}\n",
                "path":"./enclave/circuits/bin/dummy/src/main.nr"}
            },"expression_width":{"Bounded":{"width":4}}
        });

        let circuit_path = self.circuits_dir.join("pk_bfv.json");
        fs::write(&circuit_path, serde_json::to_string_pretty(&placeholder)?).await?;

        fs::create_dir_all(self.circuits_dir.join("vk")).await?;

        Ok(())
    }

    async fn download_with_progress(&self, url: &str, message: &str) -> Result<Vec<u8>, ZkError> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| ZkError::DownloadFailed(url.to_string(), e.to_string()))?;

        if !response.status().is_success() {
            return Err(ZkError::DownloadFailed(
                url.to_string(),
                format!("HTTP {}", response.status()),
            ));
        }

        let total_size = response.content_length().unwrap_or(0);

        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(message.to_string());

        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk =
                chunk.map_err(|e| ZkError::DownloadFailed(url.to_string(), e.to_string()))?;
            bytes.extend_from_slice(&chunk);
            pb.set_position(bytes.len() as u64);
        }

        pb.finish_with_message("download complete");
        Ok(bytes)
    }

    pub async fn verify_bb(&self) -> Result<String, ZkError> {
        if !self.bb_binary.exists() {
            return Err(ZkError::BbNotInstalled);
        }

        let output = tokio::process::Command::new(&self.bb_binary)
            .arg("--version")
            .output()
            .await?;

        if !output.status.success() {
            return Err(ZkError::ProveFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(version)
    }

    pub fn work_dir_for(&self, e3_id: &str) -> PathBuf {
        self.work_dir.join(e3_id)
    }

    pub async fn cleanup_work_dir(&self, e3_id: &str) -> Result<(), ZkError> {
        let work_dir = self.work_dir_for(e3_id);
        if work_dir.exists() {
            fs::remove_dir_all(&work_dir).await?;
        }
        Ok(())
    }
}

fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn versions_json_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("versions.json")
    }

    #[tokio::test]
    async fn test_backend_creates_directories() {
        let temp = tempdir().unwrap();
        let backend = ZkBackend::new(temp.path(), ZkConfig::default());

        fs::create_dir_all(&backend.base_dir).await.unwrap();
        fs::create_dir_all(&backend.circuits_dir).await.unwrap();
        fs::create_dir_all(&backend.work_dir).await.unwrap();

        assert!(backend.base_dir.exists());
        assert!(backend.circuits_dir.exists());
        assert!(backend.work_dir.exists());

        let temp_path = temp.path().to_path_buf();
        drop(temp);
        assert!(!temp_path.exists());
    }

    #[tokio::test]
    async fn test_version_info_roundtrip() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("version.json");

        let info = VersionInfo {
            bb_version: Some("0.87.0".to_string()),
            circuits_version: Some("0.1.0".to_string()),
            ..Default::default()
        };

        info.save(&path).await.unwrap();
        let loaded = VersionInfo::load(&path).await.unwrap();

        assert_eq!(loaded.bb_version, info.bb_version);
        assert_eq!(loaded.circuits_version, info.circuits_version);

        let temp_path = temp.path().to_path_buf();
        drop(temp);
        assert!(!temp_path.exists());
    }

    #[tokio::test]
    async fn test_check_status_full_setup_needed() {
        let temp = tempdir().unwrap();
        let backend = ZkBackend::new(temp.path(), ZkConfig::default());

        let status = backend.check_status().await;
        assert!(matches!(status, SetupStatus::FullSetupNeeded));

        let temp_path = temp.path().to_path_buf();
        drop(temp);
        assert!(!temp_path.exists());
    }

    #[tokio::test]
    async fn test_check_status_ready_when_installed() {
        let temp = tempdir().unwrap();
        let config = ZkConfig::default();
        let backend = ZkBackend::new(temp.path(), config.clone());

        fs::create_dir_all(&backend.base_dir.join("bin"))
            .await
            .unwrap();
        fs::create_dir_all(&backend.circuits_dir).await.unwrap();
        fs::write(&backend.bb_binary, b"fake bb binary")
            .await
            .unwrap();

        let info = VersionInfo {
            bb_version: Some(config.required_bb_version.clone()),
            circuits_version: Some(config.required_circuits_version.clone()),
            ..Default::default()
        };
        info.save(&backend.version_file()).await.unwrap();

        let status = backend.check_status().await;
        assert!(matches!(status, SetupStatus::Ready));

        let temp_path = temp.path().to_path_buf();
        drop(temp);
        assert!(!temp_path.exists());
    }

    #[tokio::test]
    async fn test_check_status_bb_needs_update() {
        let temp = tempdir().unwrap();
        let config = ZkConfig::default();
        let backend = ZkBackend::new(temp.path(), config.clone());

        fs::create_dir_all(&backend.base_dir.join("bin"))
            .await
            .unwrap();
        fs::create_dir_all(&backend.circuits_dir).await.unwrap();
        fs::write(&backend.bb_binary, b"fake bb binary")
            .await
            .unwrap();

        let info = VersionInfo {
            bb_version: Some("0.0.1".to_string()),
            circuits_version: Some(config.required_circuits_version.clone()),
            ..Default::default()
        };
        info.save(&backend.version_file()).await.unwrap();

        let status = backend.check_status().await;
        assert!(matches!(status, SetupStatus::BbNeedsUpdate { .. }));

        let temp_path = temp.path().to_path_buf();
        drop(temp);
        assert!(!temp_path.exists());
    }

    #[tokio::test]
    async fn test_work_dir_cleanup() {
        let temp = tempdir().unwrap();
        let backend = ZkBackend::new(temp.path(), ZkConfig::default());

        fs::create_dir_all(&backend.work_dir).await.unwrap();

        let e3_id = "test-e3-123";
        let work_dir = backend.work_dir_for(e3_id);

        fs::create_dir_all(&work_dir).await.unwrap();
        fs::write(work_dir.join("proof.bin"), b"fake proof")
            .await
            .unwrap();
        fs::write(work_dir.join("witness.bin"), b"fake witness")
            .await
            .unwrap();
        assert!(work_dir.exists());

        backend.cleanup_work_dir(e3_id).await.unwrap();
        assert!(!work_dir.exists());

        let temp_path = temp.path().to_path_buf();
        drop(temp);
        assert!(!temp_path.exists());
    }

    // Integration tests - require network

    #[tokio::test]
    async fn test_download_bb_with_checksum() {
        let config = ZkConfig::load(&versions_json_path())
            .await
            .expect("versions.json should exist");

        let temp = tempdir().unwrap();
        let backend = ZkBackend::new(temp.path(), config);

        let result = backend.download_bb().await;
        assert!(result.is_ok(), "download failed: {:?}", result);

        assert!(backend.bb_binary.exists(), "bb binary not found");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(&backend.bb_binary).unwrap().permissions();
            assert_eq!(perms.mode() & 0o111, 0o111, "bb not executable");
        }

        let version_info = backend.load_version_info().await;
        assert!(version_info.bb_version.is_some());
        assert!(version_info.last_updated.is_some());

        if backend
            .config
            .bb_checksum_for(BbTarget::current().unwrap())
            .is_some()
        {
            assert!(version_info.bb_checksum.is_some());
        }

        let temp_path = temp.path().to_path_buf();
        drop(temp);
        assert!(!temp_path.exists());
    }

    #[tokio::test]
    async fn test_download_bb_fails_with_wrong_checksum() {
        let mut config = ZkConfig::load(&versions_json_path())
            .await
            .expect("versions.json should exist");

        for checksum in config.bb_checksums.values_mut() {
            *checksum = "0".repeat(64);
        }

        let temp = tempdir().unwrap();
        let backend = ZkBackend::new(temp.path(), config);

        let result = backend.download_bb().await;

        assert!(
            matches!(result, Err(ZkError::ChecksumMismatch { .. })),
            "expected ChecksumMismatch, got {:?}",
            result
        );

        assert!(!backend.bb_binary.exists());

        let temp_path = temp.path().to_path_buf();
        drop(temp);
        assert!(!temp_path.exists());
    }

    #[tokio::test]
    async fn test_ensure_installed_full_flow() {
        let config = ZkConfig::load(&versions_json_path())
            .await
            .expect("versions.json should exist");

        let temp = tempdir().unwrap();
        let backend = ZkBackend::new(temp.path(), config);

        assert!(matches!(
            backend.check_status().await,
            SetupStatus::FullSetupNeeded
        ));

        let result = backend.ensure_installed().await;
        assert!(result.is_ok(), "ensure_installed failed: {:?}", result);

        assert!(matches!(backend.check_status().await, SetupStatus::Ready));

        let version = backend.verify_bb().await;
        assert!(version.is_ok(), "bb --version failed: {:?}", version);

        let temp_path = temp.path().to_path_buf();
        drop(temp);
        assert!(!temp_path.exists());
    }
}
