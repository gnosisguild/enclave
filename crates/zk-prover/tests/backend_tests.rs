// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_config::BBPath;
use e3_zk_prover::{ZkBackend, ZkConfig, ZkProver};
use tempfile::tempdir;
use tokio::fs;

fn test_backend(temp_path: &std::path::Path, config: ZkConfig) -> ZkBackend {
    let noir_dir = temp_path.join("noir");
    let bb_binary = BBPath::Default(noir_dir.join("bin").join("bb"));
    let circuits_dir = noir_dir.join("circuits");
    let work_dir = noir_dir.join("work").join("test_node");
    ZkBackend::new(bb_binary, circuits_dir, work_dir, config)
}

#[tokio::test]
async fn test_backend_creates_directories() {
    let temp = tempdir().unwrap();
    let backend = test_backend(temp.path(), ZkConfig::default());

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
async fn test_work_dir_cleanup() {
    let temp = tempdir().unwrap();
    let backend = test_backend(temp.path(), ZkConfig::default());

    fs::create_dir_all(&backend.work_dir).await.unwrap();

    let e3_id = "test-e3-123";
    let work_dir = backend.work_dir_for(e3_id).unwrap();

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

#[tokio::test]
async fn test_work_dir_path_traversal_protection() {
    let temp = tempdir().unwrap();
    let backend = test_backend(temp.path(), ZkConfig::default());

    // Test path traversal attempts
    let invalid_ids = vec!["../etc/passwd", "test/../../../etc", "test/../../secret"];

    for invalid_id in invalid_ids {
        let result = backend.cleanup_work_dir(invalid_id).await;
        assert!(
            result.is_err(),
            "Should reject path traversal: {}",
            invalid_id
        );
    }

    let result = backend.cleanup_work_dir("").await;
    assert!(result.is_err(), "Should reject empty e3_id");
    let result = backend.cleanup_work_dir("test\0bad").await;
    assert!(result.is_err(), "Should reject null byte in e3_id");

    let temp_path = temp.path().to_path_buf();
    drop(temp);
    assert!(!temp_path.exists());
}

#[test]
fn test_prover_requires_bb() {
    let temp = tempdir().unwrap();
    let backend = test_backend(temp.path(), ZkConfig::default());
    let prover = ZkProver::new(&backend);

    let result = prover.generate_proof(e3_events::CircuitName::PkBfv, b"witness", "e3-1");
    assert!(matches!(result, Err(e3_zk_prover::ZkError::BbNotInstalled)));
}
