// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NoirProverError {
    #[error("Barretenberg binary not found. Run 'enclave setup' first.")]
    BbNotInstalled,

    #[error("Circuit '{0}' not found. Run 'enclave setup' first.")]
    CircuitNotFound(String),

    #[error("Version mismatch: installed {installed}, required {required}")]
    VersionMismatch { installed: String, required: String },

    #[error("Failed to download {0}: {1}")]
    DownloadFailed(String, String),

    #[error("Checksum mismatch for {file}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    #[error("bb prove failed: {0}")]
    ProveFailed(String),

    #[error("bb verify failed: {0}")]
    VerifyFailed(String),

    #[error("Failed to serialize inputs: {0}")]
    SerializationError(String),

    #[error("Failed to read proof output: {0}")]
    OutputReadError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("TOML serialization error: {0}")]
    TomlError(#[from] toml::ser::Error),

    #[error("Setup not initialized")]
    NotInitialized,

    #[error("Unsupported platform: {os}-{arch}")]
    UnsupportedPlatform { os: String, arch: String },
}
