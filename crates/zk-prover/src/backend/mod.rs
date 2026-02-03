// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod download;
mod setup;

#[cfg(test)]
mod tests;

use crate::config::ZkConfig;
use crate::error::ZkError;
use std::path::{Path, PathBuf};

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

    pub fn work_dir_for(&self, e3_id: &str) -> PathBuf {
        self.work_dir.join(e3_id)
    }

    pub async fn cleanup_work_dir(&self, e3_id: &str) -> Result<(), ZkError> {
        // Sanitize e3_id to prevent path traversal
        if e3_id.contains("..") || e3_id.contains('/') || e3_id.contains('\\') {
            return Err(ZkError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "e3_id contains invalid characters",
            )));
        }

        let work_dir = self.work_dir_for(e3_id);
        if work_dir.exists() {
            tokio::fs::remove_dir_all(&work_dir).await?;
        }
        Ok(())
    }
}
