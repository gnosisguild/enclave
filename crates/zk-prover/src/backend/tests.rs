// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use super::*;
use crate::config::VersionInfo;
use tempfile::tempdir;
use tokio::fs;

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
