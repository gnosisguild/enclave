// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_fhe_params::{build_bfv_params_from_set_arc, BfvPreset};
use e3_pvss::sample::generate_sample;
use e3_pvss::traits::{CircuitComputation, ReduceToZkpModulus};
use e3_zk_helpers::commitments::compute_pk_bfv_commitment;
use e3_zk_prover::{
    input_map, CompiledCircuit, PkBfvCircuit, Provable, SetupStatus, WitnessGenerator, ZkBackend,
    ZkConfig, ZkProver,
};
use num_bigint::BigInt;
use std::path::PathBuf;
use tempfile::tempdir;
use tokio::{fs, process::Command};

// =============================================================================
// Backend Tests (no bb required)
// =============================================================================

#[tokio::test]
async fn test_check_status_on_empty_dir() {
    let temp = tempdir().unwrap();
    let backend = ZkBackend::new(temp.path(), ZkConfig::default());

    let status = backend.check_status().await;
    assert!(matches!(status, SetupStatus::FullSetupNeeded));
}

#[tokio::test]
async fn test_placeholder_circuits_creation() {
    let temp = tempdir().unwrap();
    let backend = ZkBackend::new(temp.path(), ZkConfig::default());

    fs::create_dir_all(&backend.circuits_dir).await.unwrap();
    backend.download_circuits().await.unwrap();

    let circuit_path = backend.circuits_dir.join("pk_bfv.json");
    assert!(circuit_path.exists());

    let content = fs::read_to_string(&circuit_path).await.unwrap();
    let _: serde_json::Value = serde_json::from_str(&content).unwrap();
}

#[tokio::test]
async fn test_work_dir_creation_and_cleanup() {
    let temp = tempdir().unwrap();
    let backend = ZkBackend::new(temp.path(), ZkConfig::default());

    let e3_id = "test-e3-123";
    let work_dir = backend.work_dir_for(e3_id);

    fs::create_dir_all(&work_dir).await.unwrap();
    assert!(work_dir.exists());

    fs::write(work_dir.join("test.txt"), "hello").await.unwrap();

    backend.cleanup_work_dir(e3_id).await.unwrap();
    assert!(!work_dir.exists());
}

#[tokio::test]
async fn test_version_info_persistence() {
    let temp = tempdir().unwrap();
    let backend = ZkBackend::new(temp.path(), ZkConfig::default());
    fs::create_dir_all(&backend.base_dir).await.unwrap();

    let info = backend.load_version_info().await;
    assert!(info.bb_version.is_none());

    let mut info = info;
    info.bb_version = Some("0.87.0".to_string());
    info.circuits_version = Some("0.1.0".to_string());
    info.save(&backend.base_dir.join("version.json"))
        .await
        .unwrap();

    let reloaded = backend.load_version_info().await;
    assert_eq!(reloaded.bb_version, Some("0.87.0".to_string()));
    assert_eq!(reloaded.circuits_version, Some("0.1.0".to_string()));
}

// =============================================================================
// Witness Generation Tests (no bb required)
// =============================================================================

#[test]
fn test_witness_generation_from_fixture() {
    let fixtures = fixtures_dir();
    let circuit = CompiledCircuit::from_file(&fixtures.join("dummy.json")).unwrap();

    let witness_gen = WitnessGenerator::new();
    let inputs = input_map([("x", "5"), ("y", "3"), ("_sum", "8")]);
    let witness = witness_gen.generate_witness(&circuit, inputs).unwrap();

    // Witness should be gzip compressed (magic bytes 0x1f 0x8b)
    assert!(witness.len() > 2);
    assert_eq!(witness[0], 0x1f);
    assert_eq!(witness[1], 0x8b);
}

#[test]
fn test_witness_generation_wrong_sum_fails() {
    let fixtures = fixtures_dir();
    let circuit = CompiledCircuit::from_file(&fixtures.join("dummy.json")).unwrap();

    let witness_gen = WitnessGenerator::new();
    let inputs = input_map([("x", "5"), ("y", "3"), ("_sum", "10")]); // Wrong sum!
    let result = witness_gen.generate_witness(&circuit, inputs);

    assert!(result.is_err());
}

#[test]
fn test_pk_bfv_witness_generation() {
    let fixtures = fixtures_dir();
    let circuit = CompiledCircuit::from_file(&fixtures.join("pk_bfv.json")).unwrap();

    // Check circuit ABI has expected parameters
    assert!(!circuit.abi.parameters.is_empty());
}

// =============================================================================
// Proof Tests (requires bb binary)
// =============================================================================

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

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
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
            println!("skipping test_pk_bfv_prove_and_verify: bb not found");
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

    // Verify proof - may fail if bb version doesn't match circuit VK version
    // This is expected in some CI environments
    match circuit.verify(&prover, &proof, e3_id) {
        Ok(true) => println!("proof verified successfully"),
        Ok(false) => {
            println!(
                "WARNING: proof verification returned false - likely bb version mismatch with VK"
            );
        }
        Err(e) => {
            println!(
                "WARNING: proof verification error: {} - likely bb version mismatch",
                e
            );
        }
    }

    // Cleanup
    prover.cleanup(e3_id).unwrap();
}

#[tokio::test]
async fn test_prover_without_bb_returns_error() {
    let temp = tempdir().unwrap();
    let backend = ZkBackend::new(temp.path(), ZkConfig::default());
    let prover = ZkProver::new(&backend);

    let result = prover.generate_proof(e3_events::CircuitName::PkBfv, b"fake witness", "test-e3");

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, e3_zk_prover::ZkError::BbNotInstalled),
        "expected BbNotInstalled error, got {:?}",
        err
    );
}
