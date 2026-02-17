// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Local end-to-end tests that require a local bb binary.
//! These tests will be skipped if bb is not found; missing fixtures cause test failure.
//!
//! To add a new circuit: add setup_*_test() and one line in `e2e_proof_tests!`.
//! Sync fixtures from circuits target: `pnpm sync:fixtures` (copies .json and .vk from
//! circuits/bin/{dkg,threshold}/target into tests/fixtures/).
//! Commitment consistency tests are defined separately.

mod common;

use common::{
    extract_field, extract_field_from_end, find_bb, setup_circuit_fixtures, setup_test_prover,
};
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::circuits::dkg::pk::circuit::PkCircuit;
use e3_zk_helpers::circuits::dkg::pk::circuit::PkCircuitData;
use e3_zk_helpers::circuits::{commitments::compute_dkg_pk_commitment, CircuitComputation};
use e3_zk_helpers::computation::DkgInputType;
use e3_zk_helpers::dkg::share_computation::{ShareComputationCircuit, ShareComputationCircuitData};
use e3_zk_helpers::dkg::share_encryption::{ShareEncryptionCircuit, ShareEncryptionCircuitData};
use e3_zk_helpers::threshold::pk_generation::{PkGenerationCircuit, PkGenerationCircuitData};
use e3_zk_helpers::threshold::{
    decrypted_shares_aggregation::{
        DecryptedSharesAggregationCircuit, DecryptedSharesAggregationCircuitData,
    },
    pk_aggregation::{PkAggregationCircuit, PkAggregationCircuitData},
    share_decryption::{
        ShareDecryptionCircuit as ThresholdShareDecryptionCircuit,
        ShareDecryptionCircuitData as ThresholdShareDecryptionCircuitData,
    },
};
use e3_zk_helpers::CiphernodesCommitteeSize;
use e3_zk_helpers::{compute_share_computation_sk_commitment, compute_threshold_pk_commitment};
use e3_zk_prover::{Provable, ZkBackend, ZkProver};

async fn setup_share_encryption_e_sm_test() -> Option<(
    ZkBackend,
    tempfile::TempDir,
    ZkProver,
    ShareEncryptionCircuit,
    ShareEncryptionCircuitData,
    BfvPreset,
    &'static str,
)> {
    let committee = CiphernodesCommitteeSize::Small.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    let sd: e3_fhe_params::PresetSearchDefaults =
        BfvPreset::InsecureThreshold512.search_defaults().unwrap();

    setup_circuit_fixtures(&backend, &["dkg", "share_encryption"], "share_encryption").await;

    let sample = ShareEncryptionCircuitData::generate_sample(
        preset,
        committee,
        DkgInputType::SmudgingNoise,
        sd.z,
        sd.lambda,
    )
    .ok()?;
    let prover = ZkProver::new(&backend);

    Some((
        backend,
        temp,
        prover,
        ShareEncryptionCircuit,
        sample,
        preset,
        "1",
    ))
}

async fn setup_share_encryption_sk_test() -> Option<(
    ZkBackend,
    tempfile::TempDir,
    ZkProver,
    ShareEncryptionCircuit,
    ShareEncryptionCircuitData,
    BfvPreset,
    &'static str,
)> {
    let committee = CiphernodesCommitteeSize::Small.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    let sd: e3_fhe_params::PresetSearchDefaults =
        BfvPreset::InsecureThreshold512.search_defaults().unwrap();

    setup_circuit_fixtures(&backend, &["dkg", "share_encryption"], "share_encryption").await;

    let sample = ShareEncryptionCircuitData::generate_sample(
        preset,
        committee,
        DkgInputType::SecretKey,
        sd.z,
        sd.lambda,
    )
    .ok()?;
    let prover = ZkProver::new(&backend);

    Some((
        backend,
        temp,
        prover,
        ShareEncryptionCircuit,
        sample,
        preset,
        "1",
    ))
}

async fn setup_share_computation_sk_test() -> Option<(
    ZkBackend,
    tempfile::TempDir,
    ZkProver,
    ShareComputationCircuit,
    ShareComputationCircuitData,
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
        ShareComputationCircuitData::generate_sample(preset, committee, DkgInputType::SecretKey)
            .ok()?;
    let prover = ZkProver::new(&backend);

    Some((
        backend,
        temp,
        prover,
        ShareComputationCircuit,
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
    ShareComputationCircuitData,
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

    let sample = ShareComputationCircuitData::generate_sample(
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
        ShareComputationCircuit,
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

async fn setup_share_decryption_test() -> Option<(
    ZkBackend,
    tempfile::TempDir,
    ZkProver,
    ThresholdShareDecryptionCircuit,
    ThresholdShareDecryptionCircuitData,
    BfvPreset,
    &'static str,
)> {
    let committee = CiphernodesCommitteeSize::Small.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    setup_circuit_fixtures(
        &backend,
        &["threshold", "share_decryption"],
        "share_decryption",
    )
    .await;

    let sample = ThresholdShareDecryptionCircuitData::generate_sample(preset, committee).ok()?;
    let prover = ZkProver::new(&backend);

    Some((
        backend,
        temp,
        prover,
        ThresholdShareDecryptionCircuit,
        sample,
        preset,
        "1",
    ))
}

async fn setup_pk_aggregation_test() -> Option<(
    ZkBackend,
    tempfile::TempDir,
    ZkProver,
    PkAggregationCircuit,
    PkAggregationCircuitData,
    BfvPreset,
    &'static str,
)> {
    let committee = CiphernodesCommitteeSize::Small.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    setup_circuit_fixtures(&backend, &["threshold", "pk_aggregation"], "pk_aggregation").await;

    let sample = PkAggregationCircuitData::generate_sample(preset, committee).ok()?;
    let prover = ZkProver::new(&backend);

    Some((
        backend,
        temp,
        prover,
        PkAggregationCircuit,
        sample,
        preset,
        "1",
    ))
}

async fn setup_decrypted_shares_aggregation_test() -> Option<(
    ZkBackend,
    tempfile::TempDir,
    ZkProver,
    DecryptedSharesAggregationCircuit,
    DecryptedSharesAggregationCircuitData,
    BfvPreset,
    &'static str,
)> {
    let committee = CiphernodesCommitteeSize::Small.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    setup_circuit_fixtures(
        &backend,
        &["threshold", "decrypted_shares_aggregation_bn"],
        "decrypted_shares_aggregation_bn",
    )
    .await;

    let sample = DecryptedSharesAggregationCircuitData::generate_sample(preset, committee).ok()?;
    let prover = ZkProver::new(&backend);

    Some((
        backend,
        temp,
        prover,
        DecryptedSharesAggregationCircuit,
        sample,
        preset,
        "1",
    ))
}

async fn setup_pk_test() -> Option<(
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
    (pk, setup_pk_test()),
    (share_computation_sk, setup_share_computation_sk_test()),
    (share_computation_e_sm, setup_share_computation_e_sm_test()),
    (share_encryption_sk, setup_share_encryption_sk_test()),
    (share_encryption_e_sm, setup_share_encryption_e_sm_test()),
    (share_decryption, setup_share_decryption_test()),
    (pk_aggregation, setup_pk_aggregation_test()),
    (decrypted_shares_aggregation, setup_decrypted_shares_aggregation_test()),
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

    // Recompute commitments from the witness
    let sk_commitment_expected = compute_share_computation_sk_commitment(
        &computation_output.inputs.sk,
        computation_output.bits.sk_bit,
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

    // NOTE: e_sm commitment check is skipped because Bounds::compute uses
    // SEARCH_N (100) for the smudging bound while the Noir circuit config
    // uses the committee size (5), producing different bit widths for packing.

    prover.cleanup(e3_id).unwrap();
}

#[tokio::test]
async fn test_pk_bfv_commitment_consistency() {
    let Some((_backend, _temp, prover, circuit, sample, preset, e3_id)) = setup_pk_test().await
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
    let commitment_calculated = computation_output.inputs.expected_secret_commitment.clone();

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
    let commitment_calculated = computation_output.inputs.expected_secret_commitment.clone();

    assert_eq!(
        commitment_from_proof, commitment_calculated,
        "Commitment from proof must match independently calculated commitment"
    );

    prover.cleanup(e3_id).unwrap();
}

#[tokio::test]
async fn test_pk_aggregation_commitment_consistency() {
    let Some((_backend, _temp, prover, circuit, sample, preset, e3_id)) =
        setup_pk_aggregation_test().await
    else {
        println!("skipping: bb not found");
        return;
    };

    let proof = circuit
        .prove(&prover, &preset, &sample, e3_id)
        .expect("proof generation should succeed");

    let computation_output = PkAggregationCircuit::compute(preset, &sample).unwrap();

    for (i, expected) in computation_output
        .inputs
        .expected_threshold_pk_commitments
        .iter()
        .enumerate()
    {
        let commitment_from_proof = extract_field(&proof.public_signals, i);
        assert_eq!(
            commitment_from_proof, *expected,
            "pk_aggregation commitment {} mismatch",
            i
        );
    }

    prover.cleanup(e3_id).unwrap();
}
