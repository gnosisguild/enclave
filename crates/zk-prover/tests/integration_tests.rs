// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Integration tests that require network access to download binaries.
//! Run with: cargo test --features integration-tests

#![cfg(feature = "integration-tests")]

use e3_zk_prover::{BbTarget, SetupStatus, ZkBackend, ZkConfig};
use std::path::PathBuf;
use tempfile::tempdir;

fn versions_json_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("versions.json")
}

#[tokio::test]
async fn test_download_bb_and_verify_structure() {
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
        assert_eq!(perms.mode() & 0o111, 0o111, "bb should be executable");
    }

    let metadata = std::fs::metadata(&backend.bb_binary).unwrap();
    assert!(metadata.len() > 0, "bb binary should not be empty");

    let version_info = backend.load_version_info().await;
    assert_eq!(
        version_info.bb_version.as_deref(),
        Some(backend.config.required_bb_version.as_str())
    );
    assert!(version_info.last_updated.is_some());

    if backend
        .config
        .bb_checksum_for(BbTarget::current().unwrap())
        .is_some()
    {
        assert!(
            version_info.bb_checksum.is_some(),
            "checksum should be saved in version.json"
        );
    }

    assert!(backend.base_dir.exists());
    assert!(backend.base_dir.join("bin").exists());

    let temp_path = temp.path().to_path_buf();
    drop(temp);
    assert!(!temp_path.exists());
}

#[tokio::test]
async fn test_download_bb_rejects_wrong_checksum() {
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
        matches!(result, Err(e3_zk_prover::ZkError::ChecksumMismatch { .. })),
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
    println!("bb version: {}", version.unwrap());

    assert!(backend.bb_binary.exists());
    assert!(backend.circuits_dir.exists());
    assert!(backend.work_dir.exists());
    assert!(backend.base_dir.join("version.json").exists());

    // Idempotent - running setup again should work
    let result = backend.ensure_installed().await;
    assert!(result.is_ok());
    assert!(matches!(backend.check_status().await, SetupStatus::Ready));

    let temp_path = temp.path().to_path_buf();
    drop(temp);
    assert!(!temp_path.exists());
}

#[tokio::test]
async fn test_download_circuits() {
    let config = ZkConfig::load(&versions_json_path())
        .await
        .expect("versions.json should exist");

    let temp = tempdir().unwrap();
    let backend = ZkBackend::new(temp.path(), config);

    tokio::fs::create_dir_all(&backend.circuits_dir)
        .await
        .unwrap();

    // Download circuits (may fall back to placeholder on failure)
    let result = backend.download_circuits().await;
    assert!(result.is_ok(), "download_circuits failed: {:?}", result);

    // Should have at least the placeholder circuit
    assert!(backend
        .circuits_dir
        .join("circuits")
        .join("dkg")
        .join("pk")
        .join("pk.json")
        .exists());
    assert!(backend
        .circuits_dir
        .join("circuits")
        .join("dkg")
        .join("pk")
        .join("pk.vk")
        .exists());

    let temp_path = temp.path().to_path_buf();
    drop(temp);
    assert!(!temp_path.exists());
}
