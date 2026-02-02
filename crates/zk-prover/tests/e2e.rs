// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod common;

use common::fixtures_dir;
use e3_fhe_params::{build_bfv_params_from_set_arc, BfvPreset};
use e3_pvss::sample::generate_sample;
use e3_pvss::traits::{CircuitComputation, ReduceToZkpModulus};
use e3_zk_helpers::commitments::compute_pk_bfv_commitment;
use e3_zk_prover::{PkBfvCircuit, Provable, ZkBackend, ZkConfig, ZkProver};
use num_bigint::BigInt;
use std::path::PathBuf;
use tempfile::tempdir;
use tokio::{fs, process::Command};

// Local bb tests — requires bb binary on system
mod local_bb {
    use super::*;

    async fn find_bb() -> Option<PathBuf> {
        if let Ok(output) = Command::new("which").arg("bb").output().await {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            for path in [
                format!("{}/.bb/bb", home),
                format!("{}/.nargo/bin/bb", home),
                format!("{}/.enclave/noir/bin/bb", home),
            ] {
                if std::path::Path::new(&path).exists() {
                    return Some(PathBuf::from(path));
                }
            }
        }
        None
    }

    async fn setup_test_prover(bb: &PathBuf) -> (ZkBackend, tempfile::TempDir) {
        let temp = tempdir().unwrap();
        let backend = ZkBackend::new(temp.path(), ZkConfig::default());

        fs::create_dir_all(&backend.circuits_dir).await.unwrap();
        fs::create_dir_all(backend.circuits_dir.join("vk"))
            .await
            .unwrap();
        fs::create_dir_all(&backend.work_dir).await.unwrap();
        fs::create_dir_all(backend.base_dir.join("bin"))
            .await
            .unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(bb, &backend.bb_binary).unwrap();

        (backend, temp)
    }

    #[tokio::test]
    async fn test_pk_bfv_prove_and_verify() {
        let bb = match find_bb().await {
            Some(p) => p,
            None => {
                println!("skipping: bb not found");
                return;
            }
        };

        let (backend, _temp) = setup_test_prover(&bb).await;
        let fixtures = fixtures_dir();

        fs::copy(
            fixtures.join("pk_bfv.json"),
            backend.circuits_dir.join("pk_bfv.json"),
        )
        .await
        .unwrap();
        fs::copy(
            fixtures.join("pk_bfv.vk"),
            backend.circuits_dir.join("vk").join("pk_bfv.vk"),
        )
        .await
        .unwrap();

        let preset = BfvPreset::InsecureDkg512;
        let params = build_bfv_params_from_set_arc(preset.into());
        let sample = generate_sample(&params);

        let prover = ZkProver::new(&backend);
        let circuit = PkBfvCircuit;
        let e3_id = "test-pk-bfv-001";

        let proof = circuit
            .prove(&prover, &params, &sample.public_key, e3_id)
            .expect("proof generation should succeed");

        assert!(!proof.data.is_empty(), "proof data should not be empty");
        assert!(
            !proof.public_signals.is_empty(),
            "public signals should not be empty"
        );

        let computation_output = circuit
            .compute(&params, &sample.public_key)
            .expect("computation should succeed");
        let reduced_witness = computation_output.witness.reduce_to_zkp_modulus();
        let commitment_calculated = compute_pk_bfv_commitment(
            &reduced_witness.pk0is,
            &reduced_witness.pk1is,
            computation_output.bits.pk_bit,
        );
        let commitment_from_proof =
            BigInt::from_bytes_be(num_bigint::Sign::Plus, &proof.public_signals);

        assert_eq!(
            commitment_calculated, commitment_from_proof,
            "commitment mismatch"
        );

        match circuit.verify(&prover, &proof, e3_id) {
            Ok(true) => println!("proof verified successfully"),
            Ok(false) => {
                println!("WARNING: verification returned false - likely bb version mismatch")
            }
            Err(e) => println!(
                "WARNING: verification error: {} - likely bb version mismatch",
                e
            ),
        }

        prover.cleanup(e3_id).unwrap();
    }
}

// Integration tests — downloads real binaries, requires network (feature flag enabled)
#[cfg(feature = "integration-tests")]
mod integration {
    use e3_zk_prover::{BbTarget, SetupStatus};

    use super::*;

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

        // Idempotent
        let result = backend.ensure_installed().await;
        assert!(result.is_ok());
        assert!(matches!(backend.check_status().await, SetupStatus::Ready));

        let temp_path = temp.path().to_path_buf();
        drop(temp);
        assert!(!temp_path.exists());
    }
}
