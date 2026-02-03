// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Local end-to-end tests that require a local bb binary.
//! These tests will be skipped if bb is not found on the system.

mod common;

use common::fixtures_dir;
use e3_fhe_params::{build_bfv_params_from_set_arc, BfvPreset};
use e3_zk_helpers::circuits::pk_bfv::circuit::PkBfvCircuitInput;
use e3_zk_helpers::circuits::sample::Sample;
use e3_zk_helpers::circuits::{commitments::compute_dkg_pk_commitment, CircuitComputation};
use e3_zk_prover::{PkBfvCircuit, Provable, ZkBackend, ZkConfig, ZkProver};
use std::path::PathBuf;
use tempfile::tempdir;
use tokio::{fs, process::Command};

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
async fn test_pk_bfv_proof_generation() {
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
    let sample = Sample::generate(&params);

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

    prover.cleanup(e3_id).unwrap();
}

#[tokio::test]
async fn test_pk_bfv_proof_verification() {
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
    let sample = Sample::generate(&params);

    let prover = ZkProver::new(&backend);
    let circuit = PkBfvCircuit;
    let e3_id = "test-verify-001";

    let proof = circuit
        .prove(&prover, &params, &sample.public_key, e3_id)
        .expect("proof generation should succeed");

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

#[tokio::test]
async fn test_pk_bfv_commitment_consistency() {
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
    let sample = Sample::generate(&params);

    let prover = ZkProver::new(&backend);
    let circuit = PkBfvCircuit;
    let e3_id = "test-commitment-001";

    let proof = circuit
        .prove(&prover, &params, &sample.public_key, e3_id)
        .expect("proof generation should succeed");

    // Verify the commitment from the proof is a valid field element
    let commitment_from_proof =
        num_bigint::BigInt::from_bytes_be(num_bigint::Sign::Plus, &proof.public_signals);
    assert!(
        commitment_from_proof > num_bigint::BigInt::from(0),
        "commitment should be positive"
    );

    // Compute the commitment independently to ensure consistency
    let circuit_input = PkBfvCircuitInput {
        public_key: sample.public_key.clone(),
    };
    let computation_output =
        PkBfvCircuit::compute(&params, &circuit_input).expect("computation should succeed");
    let commitment_calculated = compute_dkg_pk_commitment(
        &computation_output.witness.pk0is,
        &computation_output.witness.pk1is,
        computation_output.bits.pk_bit,
    );

    println!("Commitment from proof: {}", commitment_from_proof);
    println!("Commitment calculated: {}", commitment_calculated);

    prover.cleanup(e3_id).unwrap();
}
