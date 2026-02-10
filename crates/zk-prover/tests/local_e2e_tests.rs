// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Local end-to-end tests that require a local bb binary.
//! These tests will be skipped if bb is not found on the system.
//!
//! To add a new circuit: add setup_*_test() and one line in `e2e_proof_tests!`.
//! Commitment consistency tests are defined separately.

mod common;

use common::fixtures_dir;
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::circuits::dkg::pk::circuit::PkCircuit;
use e3_zk_helpers::circuits::dkg::pk::circuit::PkCircuitData;
use e3_zk_helpers::circuits::{commitments::compute_dkg_pk_commitment, CircuitComputation};
use e3_zk_helpers::threshold::pk_generation::{PkGenerationCircuit, PkGenerationCircuitData};
use e3_zk_helpers::dkg::share_computation::{ShareComputationCircuit, ShareComputationCircuitData};
use e3_zk_helpers::computation::DkgInputType;
use e3_zk_helpers::dkg::share_computation::ShareComputationCircuit;
use e3_zk_helpers::dkg::share_computation::ShareComputationCircuitInput;
use e3_zk_helpers::threshold::pk_generation::{PkGenerationCircuit, PkGenerationCircuitInput};
use e3_zk_helpers::CiphernodesCommitteeSize;
use e3_zk_helpers::{
    compute_share_computation_e_sm_commitment, compute_share_computation_sk_commitment,
    compute_threshold_pk_commitment,
};
use e3_zk_prover::{Provable, ZkBackend, ZkConfig, ZkProver};
use num_bigint::{BigInt, Sign};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::{fs, process::Command};

use crate::common::extract_field;
use crate::common::extract_field_from_end;

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
    let target_tmp = env!("CARGO_TARGET_TMPDIR");
    let temp = TempDir::new_in(target_tmp).unwrap();

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

async fn setup_circuit_fixtures(backend: &ZkBackend, circuit_path: &[&str], fixture_name: &str) {
    let circuit_dir = circuit_path
        .iter()
        .fold(backend.circuits_dir.clone(), |p, seg| p.join(seg));
    let fixtures = fixtures_dir();
    fs::create_dir_all(&circuit_dir).await.unwrap();
    fs::copy(
        fixtures.join(format!("{fixture_name}.json")),
        circuit_dir.join(format!("{fixture_name}.json")),
    )
    .await
    .unwrap();
    fs::copy(
        fixtures.join(format!("{fixture_name}.vk")),
        circuit_dir.join(format!("{fixture_name}.vk")),
    )
    .await
    .unwrap();
}

async fn setup_share_computation_sk_test() -> Option<(
    ZkBackend,
    tempfile::TempDir,
    ZkProver,
    ShareComputationCircuit,
    ShareComputationCircuitInput,
    BfvPreset,
    &'static str,
)> {
    let committee = CiphernodesCommitteeSize::Small.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    setup_circuit_fixtures(
        &backend,
        &["dkg", "sk_share_computation"],
        "sk_share_computation",
    )
    .await;

    let sample =
        ShareComputationCircuitInput::generate_sample(preset, committee, DkgInputType::SecretKey)
            .ok()?;
    let prover = ZkProver::new(&backend);

    Some((
        backend,
        temp,
        prover,
        ShareComputationCircuit::new(DkgInputType::SecretKey),
        sample,
        preset,
        "1",
    ))
}

async fn setup_share_computation_e_sm_test() -> Option<(
    ZkBackend,
    tempfile::TempDir,
    ZkProver,
    ShareComputationCircuit,
    ShareComputationCircuitInput,
    BfvPreset,
    &'static str,
)> {
    let committee = CiphernodesCommitteeSize::Small.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    setup_circuit_fixtures(
        &backend,
        &["dkg", "e_sm_share_computation"],
        "e_sm_share_computation",
    )
    .await;

    let sample = ShareComputationCircuitInput::generate_sample(
        preset,
        committee,
        DkgInputType::SmudgingNoise,
    )
    .ok()?;
    let prover = ZkProver::new(&backend);

    Some((
        backend,
        temp,
        prover,
        ShareComputationCircuit::new(DkgInputType::SmudgingNoise),
        sample,
        preset,
        "2",
    ))
}

async fn setup_pk_generation_test() -> Option<(
    ZkBackend,
    tempfile::TempDir,
    ZkProver,
    PkGenerationCircuit,
    PkGenerationCircuitData,
    BfvPreset,
    &'static str,
)> {
    let committee = CiphernodesCommitteeSize::Small.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    setup_circuit_fixtures(&backend, &["threshold", "pk_generation"], "pk_generation").await;

    let sample = PkGenerationCircuitData::generate_sample(preset, committee).ok()?;
    let prover = ZkProver::new(&backend);

    Some((
        backend,
        temp,
        prover,
        PkGenerationCircuit,
        sample,
        preset,
        "0",
    ))
}

async fn setup_pk_bfv_test() -> Option<(
    ZkBackend,
    tempfile::TempDir,
    ZkProver,
    PkCircuit,
    PkCircuitData,
    BfvPreset,
    &'static str,
)> {
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    setup_circuit_fixtures(&backend, &["dkg", "pk"], "pk").await;

    let sample = PkCircuitData::generate_sample(preset).ok()?;
    let prover = ZkProver::new(&backend);

    Some((backend, temp, prover, PkCircuit, sample, preset, "0"))
}

macro_rules! e2e_proof_tests {
    ($(($name:ident, $setup:expr)),* $(,)?) => {
        $(
            paste::paste! {
                #[tokio::test]
                async fn [<test_ $name _proof>]() {
                    let Some((_backend, _temp, prover, circuit, sample, preset, e3_id)) =
                        $setup.await
                    else {
                        println!("skipping: bb not found");
                        return;
                    };

                    let proof = circuit
                        .prove(&prover, &preset, &sample, e3_id)
                        .expect("proof generation should succeed");

                    assert!(!proof.data.is_empty(), "proof data should not be empty");
                    assert!(!proof.public_signals.is_empty(), "public signals should not be empty");

                    let party_id = 1;
                    let verification_result = circuit.verify(&prover, &proof, e3_id, party_id);
                    assert!(
                        verification_result.as_ref().is_ok_and(|&v| v),
                        "Proof verification failed: {:?}",
                        verification_result
                    );

                    prover.cleanup(e3_id).unwrap();
                }
            }
        )*
    };
}

e2e_proof_tests! {
    (pk_generation, setup_pk_generation_test()),
    (pk_bfv, setup_pk_bfv_test()),
}

#[tokio::test]
async fn test_pk_generation_commitment_consistency() {
    let Some((_backend, _temp, prover, circuit, sample, preset, e3_id)) =
        setup_pk_generation_test().await
    else {
        println!("skipping: bb not found");
        return;
    };

    let proof = circuit
        .prove(&prover, &preset, &sample, e3_id)
        .expect("proof generation should succeed");

    let computation_output = PkGenerationCircuit::compute(preset, &sample).unwrap();

    // Each commitment is represented as a single field element (32 bytes), and there are 3 commitments at the end of the public signals
    let sk_commitment_from_proof = extract_field_from_end(&proof.public_signals, 2);
    let pk_commitment_from_proof = extract_field_from_end(&proof.public_signals, 1);
    let e_sm_commitment_from_proof = extract_field_from_end(&proof.public_signals, 0);

    // Recompute commitments from the witness
    let sk_commitment_expected = compute_share_computation_sk_commitment(
        &computation_output.inputs.sk,
        computation_output.bits.sk_bit,
    );
    let e_sm_commitment_expected = compute_share_computation_e_sm_commitment(
        &computation_output.inputs.e_sm,
        computation_output.bits.e_sm_bit,
    );
    let pk_commitment_expected = compute_threshold_pk_commitment(
        &computation_output.inputs.pk0is,
        &computation_output.inputs.pk1is,
        computation_output.bits.pk_bit,
    );

    assert_eq!(
        sk_commitment_from_proof, sk_commitment_expected,
        "sk commitment mismatch"
    );
    assert_eq!(
        pk_commitment_from_proof, pk_commitment_expected,
        "pk commitment mismatch"
    );
    assert_eq!(
        e_sm_commitment_from_proof, e_sm_commitment_expected,
        "e_sm commitment mismatch"
    );

    prover.cleanup(e3_id).unwrap();
}

#[tokio::test]
async fn test_pk_bfv_commitment_consistency() {
    let Some((_backend, _temp, prover, circuit, sample, preset, e3_id)) = setup_pk_bfv_test().await
    else {
        println!("skipping: bb not found");
        return;
    };

    let proof = circuit
        .prove(&prover, &preset, &sample, e3_id)
        .expect("proof generation should succeed");

    // Verify the commitment from the proof is a valid field element
    let commitment_from_proof =
        num_bigint::BigInt::from_bytes_be(num_bigint::Sign::Plus, &proof.public_signals);
    assert!(
        commitment_from_proof > num_bigint::BigInt::from(0),
        "commitment should be positive"
    );

    // Compute the commitment independently to ensure consistency
    let computation_output =
        PkCircuit::compute(preset, &sample).expect("computation should succeed");
    let commitment_calculated = compute_dkg_pk_commitment(
        &computation_output.inputs.pk0is,
        &computation_output.inputs.pk1is,
        computation_output.bits.pk_bit,
    );

    assert_eq!(
        commitment_from_proof, commitment_calculated,
        "Commitment from proof must match independently calculated commitment"
    );

    prover.cleanup(e3_id).unwrap();
}

#[tokio::test]
async fn test_share_computation_sk_commitment_consistency() {
    let Some((_backend, _temp, prover, circuit, sample, preset, e3_id)) =
        setup_share_computation_sk_test().await
    else {
        println!("skipping: bb not found");
        return;
    };

    let proof = circuit
        .prove(&prover, &preset, &sample, e3_id)
        .expect("proof generation should succeed");

    // Verify the commitment from the proof is a valid field element
    let commitment_from_proof = extract_field(&proof.public_signals, 0);

    // Compute the commitment independently to ensure consistency
    let computation_output =
        ShareComputationCircuit::compute(preset, &sample).expect("computation should succeed");
    let commitment_calculated = computation_output
        .witness
        .expected_secret_commitment
        .clone();

    assert_eq!(
        commitment_from_proof, commitment_calculated,
        "Commitment from proof must match independently calculated commitment"
    );

    prover.cleanup(e3_id).unwrap();
}

#[tokio::test]
async fn test_share_computation_e_sm_commitment_consistency() {
    let Some((_backend, _temp, prover, circuit, sample, preset, e3_id)) =
        setup_share_computation_e_sm_test().await
    else {
        println!("skipping: bb not found");
        return;
    };

    let proof = circuit
        .prove(&prover, &preset, &sample, e3_id)
        .expect("proof generation should succeed");

    // Verify the commitment from the proof is a valid field element
    let commitment_from_proof = extract_field(&proof.public_signals, 0);

    // Compute the commitment independently to ensure consistency
    let computation_output =
        ShareComputationCircuit::compute(preset, &sample).expect("computation should succeed");
    let commitment_calculated = computation_output
        .witness
        .expected_secret_commitment
        .clone();

    assert_eq!(
        commitment_from_proof, commitment_calculated,
        "Commitment from proof must match independently calculated commitment"
    );

    prover.cleanup(e3_id).unwrap();
}
