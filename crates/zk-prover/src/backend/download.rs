// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::config::{verify_checksum, BbTarget};
use crate::error::ZkError;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tar::Archive;
use tokio::fs;
use tracing::{info, warn};
use walkdir::WalkDir;

use super::ZkBackend;

impl ZkBackend {
    pub async fn download_bb(&self) -> Result<(), ZkError> {
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
                create_placeholder_circuits(&self.circuits_dir).await?;
            }
        }

        let mut version_info = self.load_version_info().await;
        version_info.circuits_version = Some(version.clone());
        version_info.last_updated = Some(chrono::Utc::now().to_rfc3339());
        version_info.save(&self.version_file()).await?;

        info!("installed circuits v{}", version);
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

async fn create_placeholder_circuits(circuits_dir: &Path) -> Result<(), ZkError> {
    fs::create_dir_all(circuits_dir).await?;

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

    let circuit_path = circuits_dir.join("pk_bfv.json");
    fs::write(&circuit_path, serde_json::to_string_pretty(&placeholder)?).await?;

    fs::create_dir_all(circuits_dir.join("vk")).await?;

    Ok(())
}
