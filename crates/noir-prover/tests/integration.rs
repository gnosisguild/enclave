// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_noir_prover::{NoirConfig, NoirSetup, SetupStatus};
use tempfile::tempdir;
use tokio::fs;

#[tokio::test]
async fn test_check_status_on_empty_dir() {
    let temp = tempdir().unwrap();
    let setup = NoirSetup::new(temp.path(), NoirConfig::default());

    let status = setup.check_status().await;
    assert!(matches!(status, SetupStatus::FullSetupNeeded));
}

#[tokio::test]
async fn test_placeholder_circuits_creation() {
    let temp = tempdir().unwrap();
    let setup = NoirSetup::new(temp.path(), NoirConfig::default());

    fs::create_dir_all(&setup.circuits_dir).await.unwrap();

    setup.download_circuits().await.unwrap();

    let circuit_path = setup.circuits_dir.join("pk_bfv.json");
    assert!(circuit_path.exists());

    let content = fs::read_to_string(&circuit_path).await.unwrap();
    let _: serde_json::Value = serde_json::from_str(&content).unwrap();
}

#[tokio::test]
async fn test_work_dir_creation_and_cleanup() {
    let temp = tempdir().unwrap();
    let setup = NoirSetup::new(temp.path(), NoirConfig::default());

    let e3_id = "test-e3-123";
    let work_dir = setup.work_dir_for(e3_id);

    fs::create_dir_all(&work_dir).await.unwrap();
    assert!(work_dir.exists());

    fs::write(work_dir.join("test.txt"), "hello").await.unwrap();

    setup.cleanup_work_dir(e3_id).await.unwrap();
    assert!(!work_dir.exists());
}

#[tokio::test]
async fn test_version_info_persistence() {
    let temp = tempdir().unwrap();
    let setup = NoirSetup::new(temp.path(), NoirConfig::default());
    fs::create_dir_all(&setup.noir_dir).await.unwrap();

    let info = setup.load_version_info().await;
    assert!(info.bb_version.is_none());

    let mut info = info;
    info.bb_version = Some("0.87.0".to_string());
    info.circuits_version = Some("0.1.0".to_string());
    info.save(&setup.noir_dir.join("version.json"))
        .await
        .unwrap();

    let reloaded = setup.load_version_info().await;
    assert_eq!(reloaded.bb_version, Some("0.87.0".to_string()));
    assert_eq!(reloaded.circuits_version, Some("0.1.0".to_string()));
}
