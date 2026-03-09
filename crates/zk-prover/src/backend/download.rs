// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::config::{verify_checksum, BbTarget, ChecksumManifest, CircuitInfo};
use crate::error::ZkError;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tar::Archive;
use tokio::fs;
use tracing::{info, warn};
use walkdir::WalkDir;

use super::ZkBackend;

impl ZkBackend {
    pub async fn download_bb(&self) -> Result<(), ZkError> {
        if self.using_custom_bb {
            println!("IGNORING DOWNLOAD BECAUSE WE ARE USING A CUSTOM BB");
            return Ok(());
        }

        let target = BbTarget::current().ok_or_else(|| ZkError::UnsupportedPlatform {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        })?;

        let (arch, os) = target.url_parts();
        let version = &self.config.required_bb_version;

        let url = self
            .config
            .bb_download_url
            .replace("{version}", version)
            .replace("{os}", &os)
            .replace("{arch}", &arch);

        info!("downloading Barretenberg from: {}", url);

        let bytes = download_with_progress(&url, "Downloading bb").await?;
        let expected_checksum = self.config.bb_checksum_for(target);
        verify_checksum(&format!("bb-{}", target), &bytes, expected_checksum)?;

        let decoder = GzDecoder::new(&bytes[..]);
        let mut archive = Archive::new(decoder);

        let bin_dir = self.base_dir.join("bin");
        fs::create_dir_all(&bin_dir).await?;

        let temp_dir = tempfile::tempdir()?;
        archive.unpack(temp_dir.path())?;

        let bb_source = find_bb_in_dir(temp_dir.path())?;

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
        version_info.last_updated = Some(chrono::Utc::now().to_rfc3339());
        version_info.save(&self.version_file()).await?;

        info!("installed Barretenberg v{}", version);
        Ok(())
    }

    pub async fn download_circuits(&self) -> Result<(), ZkError> {
        let version = &self.config.required_circuits_version;
        let url = self
            .config
            .circuits_download_url
            .replace("{version}", version);

        info!("downloading circuits from: {}", url);

        let result = download_with_progress(&url, "Downloading circuits").await;

        let mut version_info = self.load_version_info().await;

        match result {
            Ok(bytes) => {
                if self.circuits_dir.exists() {
                    fs::remove_dir_all(&self.circuits_dir).await?;
                }

                let decoder = GzDecoder::new(&bytes[..]);
                let mut archive = Archive::new(decoder);
                archive.unpack(&self.base_dir)?;

                let circuit_infos = self.verify_circuits().await?;

                version_info.circuits = circuit_infos;
                version_info.circuits_version = Some(version.clone());
                version_info.last_updated = Some(chrono::Utc::now().to_rfc3339());
                version_info.save(&self.version_file()).await?;

                info!("installed circuits v{}", version);
            }
            Err(e) => {
                return Err(ZkError::DownloadFailed(
                    url,
                    format!("could not download circuits: {}", e),
                ));
            }
        }

        Ok(())
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

    async fn verify_circuits(&self) -> Result<HashMap<String, CircuitInfo>, ZkError> {
        let manifest_path = self.circuits_dir.join("checksums.json");
        if !manifest_path.exists() {
            warn!("checksums.json not found, skipping circuit verification");
            return Ok(HashMap::new());
        }

        let manifest_data = fs::read_to_string(&manifest_path).await?;
        let manifest: ChecksumManifest = serde_json::from_str(&manifest_data)?;

        let mut circuit_infos = HashMap::new();

        for (rel_path, expected_hash) in &manifest.files {
            let file_path = self.circuits_dir.join(rel_path);
            if !file_path.exists() {
                return Err(ZkError::CircuitNotFound(rel_path.clone()));
            }

            let data = fs::read(&file_path).await?;
            verify_checksum(rel_path, &data, Some(expected_hash))?;

            circuit_infos.insert(
                rel_path.clone(),
                CircuitInfo {
                    file: rel_path.clone(),
                    checksum: expected_hash.clone(),
                },
            );
        }

        info!(
            "verified {} circuit files from checksums.json",
            circuit_infos.len()
        );
        Ok(circuit_infos)
    }
}

fn find_bb_in_dir(dir: &Path) -> Result<PathBuf, ZkError> {
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

async fn download_with_progress(url: &str, message: &str) -> Result<Vec<u8>, ZkError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| ZkError::DownloadFailed(url.to_string(), e.to_string()))?;

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
            .template(
                "{msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(message.to_string());

    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| ZkError::DownloadFailed(url.to_string(), e.to_string()))?;
        bytes.extend_from_slice(&chunk);
        pb.set_position(bytes.len() as u64);
    }

    pb.finish_with_message("download complete");
    Ok(bytes)
}
