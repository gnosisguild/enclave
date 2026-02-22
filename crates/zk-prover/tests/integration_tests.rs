// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Integration tests that require network access to download binaries.
//! Run with: cargo test --features integration-tests

#![cfg(feature = "integration-tests")]

mod common;

use common::test_backend;
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::circuits::dkg::pk::circuit::{PkCircuit, PkCircuitData};
use e3_zk_prover::{test_utils::get_tempdir, BbTarget, Provable, SetupStatus, ZkConfig, ZkProver};
use std::{env, path::PathBuf};

fn versions_json_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("versions.json")
}

#[tokio::test]
async fn test_full_flow_download_circuits_prove_and_verify() {
    eprintln!(">>> Loading config...");
    let config = ZkConfig::load(&versions_json_path())
        .await
        .expect("versions.json should exist");
    eprintln!(">>> Config loaded");

    let temp = get_tempdir().unwrap();
    let backend = test_backend(temp.path(), config);

    if !backend.using_custom_bb {
        // --- Step 1: Fresh state should need full setup ---
        eprintln!(">>> Step 1: Checking fresh state...");
        assert!(matches!(
            backend.check_status().await,
            SetupStatus::FullSetupNeeded
        ));
        eprintln!(">>> Step 1: Done");

        // --- Step 2: Download bb and verify structure ---
        eprintln!(">>> Step 2: Downloading bb...");
        let result = backend.download_bb().await;
        eprintln!(">>> Step 2: bb downloaded");
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
    }

    let version = backend.verify_bb().await;
    assert!(version.is_ok(), "bb --version failed: {:?}", version);
    println!("bb version: {}", version.unwrap());

    // --- Step 3: Download circuits ---
    eprintln!(">>> Step 3: Downloading circuits...");
    tokio::fs::create_dir_all(&backend.circuits_dir)
        .await
        .unwrap();

    let result = backend.download_circuits().await;
    eprintln!(">>> Step 3: Circuits downloaded");
    assert!(result.is_ok(), "download_circuits failed: {:?}", result);

    assert!(backend
        .circuits_dir
        .join("dkg")
        .join("pk")
        .join("pk.json")
        .exists());
    assert!(backend
        .circuits_dir
        .join("dkg")
        .join("pk")
        .join("pk.vk")
        .exists());

    // --- Step 4: ensure_installed is idempotent on top of existing setup ---
    eprintln!(">>> Step 4: Running ensure_installed...");
    let result = backend.ensure_installed().await;
    eprintln!(">>> Step 4: ensure_installed done");
    assert!(result.is_ok(), "ensure_installed failed: {:?}", result);
    assert!(matches!(backend.check_status().await, SetupStatus::Ready));

    assert!(backend.bb_binary.exists());
    assert!(backend.circuits_dir.exists());
    assert!(backend.work_dir.exists());
    assert!(backend.base_dir.join("version.json").exists());

    // --- Step 5: Generate and verify a proof ---
    eprintln!(">>> Step 5: Generating proof...");
    let preset = BfvPreset::InsecureThreshold512;
    let prover = ZkProver::new(&backend);

    let sample =
        PkCircuitData::generate_sample(preset).expect("sample data generation should succeed");

    let e3_id = "integration-test-full-flow";
    let proof = PkCircuit
        .prove(&prover, &preset, &sample, e3_id)
        .expect("proof generation should succeed");

    eprintln!(">>> Step 5: Proof generated");
    assert!(!proof.data.is_empty(), "proof data should not be empty");
    assert!(
        !proof.public_signals.is_empty(),
        "public signals should not be empty"
    );

    eprintln!(">>> Step 5: Verifying proof...");
    let party_id = 0;
    let verified = PkCircuit
        .verify(&prover, &proof, e3_id, party_id)
        .expect("verification call should not error");

    assert!(verified, "proof should verify successfully");
    eprintln!(">>> Step 5: Proof verified!");

    prover.cleanup(e3_id).unwrap();

    // --- Cleanup ---
    let temp_path = temp.path().to_path_buf();
    drop(temp);
    assert!(!temp_path.exists());
}

#[tokio::test]
async fn test_download_bb_rejects_wrong_checksum() {
    if env::var("E3_CUSTOM_BB").is_ok() {
        return;
    }

    let mut config = ZkConfig::load(&versions_json_path())
        .await
        .expect("versions.json should exist");

    for checksum in config.bb_checksums.values_mut() {
        *checksum = "0".repeat(64);
    }

    let temp = get_tempdir().unwrap();
    let backend = test_backend(temp.path(), config);

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
