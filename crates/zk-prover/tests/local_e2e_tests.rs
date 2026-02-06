// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Local end-to-end tests that require a local bb binary.
//! These tests will be skipped if bb is not found on the system.

mod common;

use common::fixtures_dir;
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::ciphernodes_committee::CiphernodesCommitteeSize;
use e3_zk_helpers::circuits::dkg::pk::circuit::PkCircuitInput;
use e3_zk_helpers::circuits::dkg::pk::prepare_pk_sample_for_test;
use e3_zk_helpers::circuits::{commitments::compute_dkg_pk_commitment, CircuitComputation};
use e3_zk_helpers::PkCircuit;
use e3_zk_prover::{Provable, ZkBackend, ZkConfig, ZkProver};
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
    let temp_path = temp.path();
    let noir_dir = temp_path.join("noir");
    let bb_binary = noir_dir.join("bin").join("bb");
    let circuits_dir = noir_dir.join("circuits");
    let work_dir = noir_dir.join("work").join("test_node");
    let backend = ZkBackend::new(
        bb_binary.clone(),
        circuits_dir.clone(),
        work_dir.clone(),
        ZkConfig::default(),
    );

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

    let circuit_dir = backend.circuits_dir.join("dkg").join("pk");
    fs::create_dir_all(&circuit_dir).await.unwrap();
    fs::copy(fixtures.join("pk.json"), circuit_dir.join("pk.json"))
        .await
        .unwrap();
    fs::copy(fixtures.join("pk.vk"), circuit_dir.join("pk.vk"))
        .await
        .unwrap();

    let preset = BfvPreset::InsecureThreshold512;
    let sample = prepare_pk_sample_for_test(preset, CiphernodesCommitteeSize::Small);

    let prover = ZkProver::new(&backend);
    let circuit = PkCircuit;
    let e3_id = "test-pk-bfv-001";

    let proof = circuit
        .prove(&prover, &preset, &sample.dkg_public_key, e3_id)
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

    let circuit_dir = backend.circuits_dir.join("dkg").join("pk");
    fs::create_dir_all(&circuit_dir).await.unwrap();
    fs::copy(fixtures.join("pk.json"), circuit_dir.join("pk.json"))
        .await
        .unwrap();
    fs::copy(fixtures.join("pk.vk"), circuit_dir.join("pk.vk"))
        .await
        .unwrap();

    let preset = BfvPreset::InsecureThreshold512;
    let sample = prepare_pk_sample_for_test(preset, CiphernodesCommitteeSize::Small);

    let prover = ZkProver::new(&backend);
    let circuit = PkCircuit;
    let e3_id = "test-verify-001";

    let proof = circuit
        .prove(&prover, &preset, &sample.dkg_public_key, e3_id)
        .expect("proof generation should succeed");

    let verification_result = circuit.verify(&prover, &proof, e3_id);
    assert!(
        verification_result.as_ref().is_ok_and(|&v| v),
        "Proof verification failed: {:?}",
        verification_result
    );

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

    let circuit_dir = backend.circuits_dir.join("dkg").join("pk");
    fs::create_dir_all(&circuit_dir).await.unwrap();
    fs::copy(fixtures.join("pk.json"), circuit_dir.join("pk.json"))
        .await
        .unwrap();
    fs::copy(fixtures.join("pk.vk"), circuit_dir.join("pk.vk"))
        .await
        .unwrap();

    let preset = BfvPreset::InsecureThreshold512;
    let sample = prepare_pk_sample_for_test(preset, CiphernodesCommitteeSize::Small);

    let prover = ZkProver::new(&backend);
    let circuit = PkCircuit;
    let e3_id = "test-commitment-001";

    let proof = circuit
        .prove(&prover, &preset, &sample.dkg_public_key, e3_id)
        .expect("proof generation should succeed");

    // Verify the commitment from the proof is a valid field element
    let commitment_from_proof =
        num_bigint::BigInt::from_bytes_be(num_bigint::Sign::Plus, &proof.public_signals);
    assert!(
        commitment_from_proof > num_bigint::BigInt::from(0),
        "commitment should be positive"
    );

    // Compute the commitment independently to ensure consistency
    let circuit_input = PkCircuitInput {
        public_key: sample.dkg_public_key.clone(),
    };
    let computation_output =
        PkCircuit::compute(preset, &circuit_input).expect("computation should succeed");
    let commitment_calculated = compute_dkg_pk_commitment(
        &computation_output.witness.pk0is,
        &computation_output.witness.pk1is,
        computation_output.bits.pk_bit,
    );

    println!("Commitment from proof: {}", commitment_from_proof);
    println!("Commitment calculated: {}", commitment_calculated);

    assert_eq!(
        commitment_from_proof, commitment_calculated,
        "Commitment from proof must match independently calculated commitment"
    );

    prover.cleanup(e3_id).unwrap();
}
