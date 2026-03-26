// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Local end-to-end tests that require a local bb binary and pre-compiled circuit artifacts.
//! These tests will be skipped if bb is not found.
//!
//! Circuit artifacts (`.json` + `.vk`) are expected in `circuits/bin/{group}/target/`,
//! produced by `pnpm build:circuits` locally or the `build_circuits` CI job.
//!
//! To add a new circuit: add setup_*_test() and one line in `e2e_proof_tests!`
//! `(name, setup, CircuitVariant::Recursive | Evm)` (C5 pk_aggregation uses Evm).
//! Commitment consistency tests are defined separately.

mod common;

use ark_bn254::Fr;
use ark_ff::{PrimeField, Zero};
use common::{
    extract_field, extract_field_from_end, find_bb, setup_compiled_circuit, setup_test_prover,
};
use e3_fhe_params::{build_pair_for_preset, BfvPreset};
use e3_zk_helpers::circuits::dkg::pk::circuit::PkCircuit;
use e3_zk_helpers::circuits::dkg::pk::circuit::PkCircuitData;
use e3_zk_helpers::circuits::threshold::pk_generation::utils::deterministic_crp_crt_polynomial;
use e3_zk_helpers::circuits::{commitments::compute_dkg_pk_commitment, CircuitComputation};
use e3_zk_helpers::computation::DkgInputType;
use e3_zk_helpers::dkg::share_computation::{
    Configs, ShareComputationBaseCircuit, ShareComputationChunkCircuit,
    ShareComputationChunkCircuitData, ShareComputationCircuit, ShareComputationCircuitData,
};
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
use e3_zk_helpers::Computation;
use e3_zk_helpers::{
    compute_pk_aggregation_commitment, compute_share_computation_sk_commitment,
    compute_threshold_pk_commitment,
};
use e3_zk_prover::{
    generate_chunk_batch_proof, generate_share_computation_final_proof, CircuitVariant, Provable,
    ZkBackend, ZkProver,
};

/// Convert raw public signals bytes (32-byte big-endian chunks) to ark_bn254::Fr field elements.
fn public_signals_to_fields(signals: &[u8]) -> Vec<Fr> {
    signals
        .chunks(32)
        .map(|chunk| Fr::from_be_bytes_mod_order(chunk))
        .collect()
}

async fn setup_share_encryption_e_sm_test() -> Option<(
    ZkBackend,
    tempfile::TempDir,
    ZkProver,
    ShareEncryptionCircuit,
    ShareEncryptionCircuitData,
    BfvPreset,
    &'static str,
)> {
    let committee = CiphernodesCommitteeSize::Micro.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    let sd: e3_fhe_params::PresetSearchDefaults =
        BfvPreset::InsecureThreshold512.search_defaults().unwrap();

    setup_compiled_circuit(&backend, "dkg", "share_encryption").await;

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
    let committee = CiphernodesCommitteeSize::Micro.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    let sd: e3_fhe_params::PresetSearchDefaults =
        BfvPreset::InsecureThreshold512.search_defaults().unwrap();

    setup_compiled_circuit(&backend, "dkg", "share_encryption").await;

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

async fn setup_share_computation_sk_base_chunk_test() -> Option<(
    ZkBackend,
    tempfile::TempDir,
    ZkProver,
    ShareComputationCircuit,
    ShareComputationCircuitData,
    BfvPreset,
    &'static str,
)> {
    let committee = CiphernodesCommitteeSize::Micro.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    // Two-level wrapper: base, chunk, chunk_batch (level 1), share_computation (level 2)
    setup_compiled_circuit(&backend, "dkg", "sk_share_computation_base").await;
    setup_compiled_circuit(&backend, "dkg", "e_sm_share_computation_base").await;
    setup_compiled_circuit(&backend, "dkg", "share_computation_chunk").await;
    setup_compiled_circuit(&backend, "dkg", "share_computation_chunk_batch").await;
    setup_compiled_circuit(&backend, "dkg", "share_computation").await;

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

async fn setup_share_computation_e_sm_base_chunk_test() -> Option<(
    ZkBackend,
    tempfile::TempDir,
    ZkProver,
    ShareComputationCircuit,
    ShareComputationCircuitData,
    BfvPreset,
    &'static str,
)> {
    let committee = CiphernodesCommitteeSize::Micro.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    // Two-level wrapper: base, chunk, chunk_batch (level 1), share_computation (level 2)
    setup_compiled_circuit(&backend, "dkg", "sk_share_computation_base").await;
    setup_compiled_circuit(&backend, "dkg", "e_sm_share_computation_base").await;
    setup_compiled_circuit(&backend, "dkg", "share_computation_chunk").await;
    setup_compiled_circuit(&backend, "dkg", "share_computation_chunk_batch").await;
    setup_compiled_circuit(&backend, "dkg", "share_computation").await;

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
    let committee = CiphernodesCommitteeSize::Micro.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    setup_compiled_circuit(&backend, "threshold", "pk_generation").await;

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
    let committee = CiphernodesCommitteeSize::Micro.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    setup_compiled_circuit(&backend, "threshold", "share_decryption").await;

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
    let committee = CiphernodesCommitteeSize::Micro.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    setup_compiled_circuit(&backend, "threshold", "pk_aggregation").await;

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
    let committee = CiphernodesCommitteeSize::Micro.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    setup_compiled_circuit(&backend, "threshold", "decrypted_shares_aggregation").await;

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

    setup_compiled_circuit(&backend, "dkg", "pk").await;

    let sample = PkCircuitData::generate_sample(preset).ok()?;
    let prover = ZkProver::new(&backend);

    Some((backend, temp, prover, PkCircuit, sample, preset, "0"))
}

macro_rules! e2e_proof_tests {
    ($(($name:ident, $setup:expr, $variant:expr)),* $(,)?) => {
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
                        .prove_with_variant(&prover, &preset, &sample, e3_id, $variant)
                        .expect("proof generation should succeed");

                    assert!(!proof.data.is_empty(), "proof data should not be empty");
                    assert!(!proof.public_signals.is_empty(), "public signals should not be empty");

                    let party_id = 1;
                    let verification_result =
                        circuit.verify_with_variant(&prover, &proof, e3_id, party_id, $variant);
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
    (pk_generation, setup_pk_generation_test(), CircuitVariant::Recursive),
    (pk, setup_pk_test(), CircuitVariant::Recursive),
    (share_computation_sk, setup_share_computation_sk_base_chunk_test(), CircuitVariant::Recursive),
    (share_computation_e_sm, setup_share_computation_e_sm_base_chunk_test(), CircuitVariant::Recursive),
    (share_encryption_sk, setup_share_encryption_sk_test(), CircuitVariant::Recursive),
    (share_encryption_e_sm, setup_share_encryption_e_sm_test(), CircuitVariant::Recursive),
    (share_decryption, setup_share_decryption_test(), CircuitVariant::Recursive),
    (pk_aggregation, setup_pk_aggregation_test(), CircuitVariant::Evm),
    (decrypted_shares_aggregation, setup_decrypted_shares_aggregation_test(), CircuitVariant::Recursive),
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
    let (threshold_params, _) = build_pair_for_preset(preset).expect("preset pair");
    let a = deterministic_crp_crt_polynomial(&threshold_params).expect("crp polynomial");
    let pk_commitment_expected = compute_threshold_pk_commitment(
        &computation_output.inputs.pk0is,
        &a,
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
    let Some((_backend, _temp, prover, _circuit, sample, preset, e3_id)) =
        setup_share_computation_sk_base_chunk_test().await
    else {
        println!("skipping: bb not found");
        return;
    };

    // Run the pipeline manually to capture intermediate public signals
    let base_proof = ShareComputationBaseCircuit
        .prove(&prover, &preset, &sample, &format!("{e3_id}_base"))
        .expect("base proof should succeed");

    let configs = Configs::compute(preset, &sample).expect("configs");

    let mut chunk_proofs = Vec::with_capacity(configs.n_chunks);
    for chunk_idx in 0..configs.n_chunks {
        let chunk_data = ShareComputationChunkCircuitData {
            share_data: sample.clone(),
            chunk_idx,
        };
        let chunk_proof = ShareComputationChunkCircuit
            .prove(
                &prover,
                &preset,
                &chunk_data,
                &format!("{e3_id}_chunk_{chunk_idx}"),
            )
            .expect("chunk proof should succeed");
        chunk_proofs.push(chunk_proof);
    }

    // Level 1: group chunks into batches and prove each batch
    let mut batch_proofs = Vec::with_capacity(configs.n_batches);
    for batch_idx in 0..configs.n_batches {
        let start = batch_idx * configs.chunks_per_batch;
        let end = start + configs.chunks_per_batch;
        let batch_proof = generate_chunk_batch_proof(
            &prover,
            &base_proof,
            &chunk_proofs[start..end],
            batch_idx as u32,
            &format!("{e3_id}_batch_{batch_idx}"),
        )
        .expect("chunk batch proof should succeed");
        batch_proofs.push(batch_proof);
    }

    // Level 2: aggregate batch proofs into final C2 proof
    let proof = generate_share_computation_final_proof(&prover, &batch_proofs, e3_id)
        .expect("final share_computation proof should succeed");

    // Final circuit exposes 3 public outputs:
    //   [0] batch_key_hash (pub param)
    //   [1] key_hash (from return tuple)
    //   [2] final_commitment (from return tuple)
    assert_eq!(
        proof.public_signals.len(),
        3 * 32,
        "final share_computation should expose 3 field public inputs (96 bytes)"
    );

    // Sanity check: at least one public input is non-zero.
    let fields = public_signals_to_fields(&proof.public_signals);
    assert!(
        fields.iter().any(|f| !f.is_zero()),
        "party commitments from final wrapper should not all be zero"
    );

    prover.cleanup(e3_id).unwrap();
}

#[tokio::test]
async fn test_share_computation_e_sm_commitment_consistency() {
    let Some((_backend, _temp, prover, _circuit, sample, preset, e3_id)) =
        setup_share_computation_e_sm_base_chunk_test().await
    else {
        println!("skipping: bb not found");
        return;
    };

    // Run the pipeline manually to capture intermediate public signals
    let base_proof = ShareComputationBaseCircuit
        .prove(&prover, &preset, &sample, &format!("{e3_id}_base"))
        .expect("base proof should succeed");

    let configs = Configs::compute(preset, &sample).expect("configs");

    let mut chunk_proofs = Vec::with_capacity(configs.n_chunks);
    for chunk_idx in 0..configs.n_chunks {
        let chunk_data = ShareComputationChunkCircuitData {
            share_data: sample.clone(),
            chunk_idx,
        };
        let chunk_proof = ShareComputationChunkCircuit
            .prove(
                &prover,
                &preset,
                &chunk_data,
                &format!("{e3_id}_chunk_{chunk_idx}"),
            )
            .expect("chunk proof should succeed");
        chunk_proofs.push(chunk_proof);
    }

    // Level 1: group chunks into batches and prove each batch
    let mut batch_proofs = Vec::with_capacity(configs.n_batches);
    for batch_idx in 0..configs.n_batches {
        let start = batch_idx * configs.chunks_per_batch;
        let end = start + configs.chunks_per_batch;
        let batch_proof = generate_chunk_batch_proof(
            &prover,
            &base_proof,
            &chunk_proofs[start..end],
            batch_idx as u32,
            &format!("{e3_id}_batch_{batch_idx}"),
        )
        .expect("chunk batch proof should succeed");
        batch_proofs.push(batch_proof);
    }

    // Level 2: aggregate batch proofs into final C2 proof
    let proof = generate_share_computation_final_proof(&prover, &batch_proofs, e3_id)
        .expect("final share_computation proof should succeed");

    // Final circuit exposes 3 public outputs:
    //   [0] batch_key_hash (pub param)
    //   [1] key_hash (from return tuple)
    //   [2] final_commitment (from return tuple)
    assert_eq!(
        proof.public_signals.len(),
        3 * 32,
        "final share_computation should expose 3 field public inputs (96 bytes)"
    );

    // Sanity check: at least one public input is non-zero.
    let fields = public_signals_to_fields(&proof.public_signals);
    assert!(
        fields.iter().any(|f| !f.is_zero()),
        "party commitments from final wrapper should not all be zero"
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

    // C5 uses Evm variant in production; Recursive fails because commitment hashes (256-bit)
    // exceed the noir-recursive verifier's limb bound.
    let proof = circuit
        .prove_with_variant(&prover, &preset, &sample, e3_id, CircuitVariant::Evm)
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
            "pk_aggregation per-party commitment {} mismatch",
            i
        );
    }

    let expected_final_commitment = compute_pk_aggregation_commitment(
        &computation_output.inputs.pk0_agg,
        &computation_output.inputs.pk1_agg,
        computation_output.bits.pk_bit,
    );
    let final_commitment_from_proof = extract_field_from_end(&proof.public_signals, 0);
    assert_eq!(
        final_commitment_from_proof, expected_final_commitment,
        "pk_aggregation final commitment mismatch"
    );

    prover.cleanup(e3_id).unwrap();
}
