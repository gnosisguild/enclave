// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Production witness builders and provers for the per-node DKG fold pipeline and aggregator
//! proofs ([`CircuitName::NodeFold`], [`CircuitName::DkgAggregator`], [`CircuitName::DecryptionAggregator`]).

use crate::circuits::aggregation::c3_accumulator::generate_sequential_c3_fold;
use crate::circuits::aggregation::c6_accumulator::generate_sequential_c6_fold;
use crate::circuits::aggregation::helpers::u64_to_field_hex;
use crate::circuits::aggregation::nodes_fold_accumulator::generate_sequential_nodes_fold;
use crate::circuits::utils::{bytes_to_field_strings, inputs_json_to_input_map};
use crate::circuits::vk;
use crate::error::ZkError;
use crate::prover::ZkProver;
use crate::witness::{CompiledCircuit, WitnessGenerator};
use e3_events::{CircuitName, CircuitVariant, Proof};
use serde::Serialize;

fn proof_field_strings(proof: &Proof) -> Result<Vec<String>, ZkError> {
    bytes_to_field_strings(proof.data.as_ref())
}

fn proof_public_field_strings(proof: &Proof) -> Result<Vec<String>, ZkError> {
    bytes_to_field_strings(proof.public_signals.as_ref())
}

/// Load `<artifacts_dir>/<variant>/<circuit>/<circuit>.json` as a [`CompiledCircuit`]. Centralised
/// so the layout is in one place across all aggregation builders below.
fn load_compiled_circuit(
    prover: &ZkProver,
    circuit: CircuitName,
    variant: CircuitVariant,
    artifacts_dir: &str,
) -> Result<CompiledCircuit, ZkError> {
    let path = prover
        .circuits_dir(variant, artifacts_dir)
        .join(circuit.dir_path())
        .join(format!("{}.json", circuit.as_str()));
    CompiledCircuit::from_file(&path)
}

/// Build a witness from a `Serialize` Noir input struct and prove `circuit` via the recursive-bin
/// path. Collapses the repeated `serde_json::to_value → inputs_json_to_input_map → CompiledCircuit::from_file
/// → WitnessGenerator → generate_recursive_aggregation_bin_proof` boilerplate used by every fold
/// builder (c2ab / c3ab / c4ab / node_fold) in this module.
fn build_and_prove_recursive_bin<W: Serialize>(
    prover: &ZkProver,
    circuit: CircuitName,
    witness_input: &W,
    job_label: &str,
    artifacts_dir: &str,
) -> Result<Proof, ZkError> {
    let json = serde_json::to_value(witness_input)
        .map_err(|e| ZkError::SerializationError(e.to_string()))?;
    let input_map = inputs_json_to_input_map(&json)?;
    let compiled = load_compiled_circuit(prover, circuit, CircuitVariant::Default, artifacts_dir)?;
    let witness = WitnessGenerator::new().generate_witness(&compiled, input_map)?;
    prover.generate_recursive_aggregation_bin_proof(circuit, &witness, job_label, artifacts_dir)
}

#[derive(Serialize)]
struct C2abFoldWitness {
    c2a_vk: Vec<String>,
    c2a_proof: Vec<String>,
    c2a_public: Vec<String>,
    c2b_vk: Vec<String>,
    c2b_proof: Vec<String>,
    c2b_public: Vec<String>,
    c2a_key_hash: String,
    c2b_key_hash: String,
}

#[derive(Serialize)]
struct C3abFoldWitness {
    c3a_vk: Vec<String>,
    c3a_proof: Vec<String>,
    c3a_public: Vec<String>,
    c3b_vk: Vec<String>,
    c3b_proof: Vec<String>,
    c3b_public: Vec<String>,
    c3a_key_hash: String,
    c3b_key_hash: String,
}

#[derive(Serialize)]
struct C4abFoldWitness {
    c4a_vk: Vec<String>,
    c4a_proof: Vec<String>,
    c4a_public: Vec<String>,
    c4b_vk: Vec<String>,
    c4b_proof: Vec<String>,
    c4b_public: Vec<String>,
    c4a_key_hash: String,
    c4b_key_hash: String,
}

#[derive(Serialize)]
struct NodeFoldWitness {
    c0_vk: Vec<String>,
    c0_proof: Vec<String>,
    c0_public: Vec<String>,
    c1_vk: Vec<String>,
    c1_proof: Vec<String>,
    c1_public: Vec<String>,
    c2ab_vk: Vec<String>,
    c2ab_proof: Vec<String>,
    c2ab_public: Vec<String>,
    c3ab_vk: Vec<String>,
    c3ab_proof: Vec<String>,
    c3ab_public: Vec<String>,
    c4ab_vk: Vec<String>,
    c4ab_proof: Vec<String>,
    c4ab_public: Vec<String>,
    party_id: String,
    c0_key_hash: String,
    c1_key_hash: String,
    c2ab_key_hash: String,
    c3ab_key_hash: String,
    c4ab_key_hash: String,
}

/// Inputs for [`prove_node_dkg_fold`]: recursive inner proofs and C3 slot metadata.
pub struct NodeDkgFoldInput<'a> {
    pub c0_proof: &'a Proof,
    pub c1_proof: &'a Proof,
    pub c2a_proof: &'a Proof,
    pub c2b_proof: &'a Proof,
    pub c3a_inner_proofs: &'a [Proof],
    pub c3b_inner_proofs: &'a [Proof],
    pub c3_slot_indices_a: &'a [u32],
    pub c3_slot_indices_b: &'a [u32],
    pub c3_total_slots: usize,
    pub c4a_proof: &'a Proof,
    pub c4b_proof: &'a Proof,
    pub party_id: u64,
}

/// Run C2abFold → C3 folds → C3abFold → C4abFold → NodeFold; returns a [`CircuitName::NodeFold`] proof.
pub fn prove_node_dkg_fold(
    prover: &ZkProver,
    input: &NodeDkgFoldInput,
    e3_id: &str,
    artifacts_dir: &str,
) -> Result<Proof, ZkError> {
    let c2a_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Recursive, artifacts_dir),
        CircuitName::SkShareComputation,
    )?;
    let c2b_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Recursive, artifacts_dir),
        CircuitName::ESmShareComputation,
    )?;

    let c2ab = C2abFoldWitness {
        c2a_vk: c2a_vk.verification_key.clone(),
        c2a_proof: proof_field_strings(input.c2a_proof)?,
        c2a_public: proof_public_field_strings(input.c2a_proof)?,
        c2b_vk: c2b_vk.verification_key.clone(),
        c2b_proof: proof_field_strings(input.c2b_proof)?,
        c2b_public: proof_public_field_strings(input.c2b_proof)?,
        c2a_key_hash: c2a_vk.key_hash.clone(),
        c2b_key_hash: c2b_vk.key_hash.clone(),
    };
    let c2ab_proof = build_and_prove_recursive_bin(
        prover,
        CircuitName::C2abFold,
        &c2ab,
        &format!("{e3_id}-c2ab"),
        artifacts_dir,
    )?;

    let c3a_folded = generate_sequential_c3_fold(
        prover,
        input.c3a_inner_proofs,
        input.c3_slot_indices_a,
        input.c3_total_slots,
        &format!("{e3_id}-c3a"),
        artifacts_dir,
    )?;
    let c3b_folded = generate_sequential_c3_fold(
        prover,
        input.c3b_inner_proofs,
        input.c3_slot_indices_b,
        input.c3_total_slots,
        &format!("{e3_id}-c3b"),
        artifacts_dir,
    )?;

    let c3_fold_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        CircuitName::C3Fold,
    )?;
    let c3ab = C3abFoldWitness {
        c3a_vk: c3_fold_vk.verification_key.clone(),
        c3a_proof: proof_field_strings(&c3a_folded)?,
        c3a_public: proof_public_field_strings(&c3a_folded)?,
        c3b_vk: c3_fold_vk.verification_key.clone(),
        c3b_proof: proof_field_strings(&c3b_folded)?,
        c3b_public: proof_public_field_strings(&c3b_folded)?,
        c3a_key_hash: c3_fold_vk.key_hash.clone(),
        c3b_key_hash: c3_fold_vk.key_hash.clone(),
    };
    let c3ab_proof = build_and_prove_recursive_bin(
        prover,
        CircuitName::C3abFold,
        &c3ab,
        &format!("{e3_id}-c3ab"),
        artifacts_dir,
    )?;

    // C4a and C4b are both proofs of the same `DkgShareDecryption` circuit, so they share the
    // same VK. Load it once and clone into both witness slots.
    let c4_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Recursive, artifacts_dir),
        CircuitName::DkgShareDecryption,
    )?;
    let c4ab = C4abFoldWitness {
        c4a_vk: c4_vk.verification_key.clone(),
        c4a_proof: proof_field_strings(input.c4a_proof)?,
        c4a_public: proof_public_field_strings(input.c4a_proof)?,
        c4b_vk: c4_vk.verification_key.clone(),
        c4b_proof: proof_field_strings(input.c4b_proof)?,
        c4b_public: proof_public_field_strings(input.c4b_proof)?,
        c4a_key_hash: c4_vk.key_hash.clone(),
        c4b_key_hash: c4_vk.key_hash.clone(),
    };
    let c4ab_proof = build_and_prove_recursive_bin(
        prover,
        CircuitName::C4abFold,
        &c4ab,
        &format!("{e3_id}-c4ab"),
        artifacts_dir,
    )?;

    let c0_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Recursive, artifacts_dir),
        CircuitName::PkBfv,
    )?;
    let c1_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Recursive, artifacts_dir),
        CircuitName::PkGeneration,
    )?;
    let c2ab_fold_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        CircuitName::C2abFold,
    )?;
    let c3ab_fold_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        CircuitName::C3abFold,
    )?;
    let c4ab_fold_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        CircuitName::C4abFold,
    )?;

    let nf = NodeFoldWitness {
        c0_vk: c0_vk.verification_key,
        c0_proof: proof_field_strings(input.c0_proof)?,
        c0_public: proof_public_field_strings(input.c0_proof)?,
        c1_vk: c1_vk.verification_key,
        c1_proof: proof_field_strings(input.c1_proof)?,
        c1_public: proof_public_field_strings(input.c1_proof)?,
        c2ab_vk: c2ab_fold_vk.verification_key,
        c2ab_proof: proof_field_strings(&c2ab_proof)?,
        c2ab_public: proof_public_field_strings(&c2ab_proof)?,
        c3ab_vk: c3ab_fold_vk.verification_key,
        c3ab_proof: proof_field_strings(&c3ab_proof)?,
        c3ab_public: proof_public_field_strings(&c3ab_proof)?,
        c4ab_vk: c4ab_fold_vk.verification_key,
        c4ab_proof: proof_field_strings(&c4ab_proof)?,
        c4ab_public: proof_public_field_strings(&c4ab_proof)?,
        party_id: u64_to_field_hex(input.party_id),
        c0_key_hash: c0_vk.key_hash,
        c1_key_hash: c1_vk.key_hash,
        c2ab_key_hash: c2ab_fold_vk.key_hash,
        c3ab_key_hash: c3ab_fold_vk.key_hash,
        c4ab_key_hash: c4ab_fold_vk.key_hash,
    };

    build_and_prove_recursive_bin(
        prover,
        CircuitName::NodeFold,
        &nf,
        &format!("{e3_id}-nodefold"),
        artifacts_dir,
    )
}

/// Inputs for [`prove_dkg_aggregation`].
pub struct DkgAggregationInput<'a> {
    pub node_fold_proofs: &'a [Proof],
    pub c5_proof: &'a Proof,
    /// Honest party ids in the same order as `node_fold_proofs` (e.g. sorted ascending).
    pub party_ids: &'a [u64],
}

#[derive(Serialize)]
struct DkgAggregatorWitness {
    nodes_fold_vk: Vec<String>,
    nodes_fold_proof: Vec<String>,
    nodes_fold_public: Vec<String>,
    c5_vk: Vec<String>,
    c5_proof: Vec<String>,
    c5_public: Vec<String>,
    nodes_fold_key_hash: String,
    c5_key_hash: String,
    party_ids: Vec<String>,
}

/// [`CircuitName::DkgAggregator`] over sequential [`CircuitName::NodesFold`] + C5, proved with
/// [`CircuitVariant::Evm`] for on-chain verification.
pub fn prove_dkg_aggregation(
    prover: &ZkProver,
    input: &DkgAggregationInput,
    e3_id: &str,
    artifacts_dir: &str,
) -> Result<Proof, ZkError> {
    if input.node_fold_proofs.len() != input.party_ids.len() {
        return Err(ZkError::InvalidInput(
            "node_fold_proofs and party_ids length mismatch".into(),
        ));
    }
    if input.node_fold_proofs.is_empty() {
        return Err(ZkError::InvalidInput(
            "prove_dkg_aggregation: need at least one NodeFold proof".into(),
        ));
    }
    let h = input.node_fold_proofs.len();
    let slot_indices: Vec<u32> = (0u32..h as u32).collect();
    let nodes_fold_proof = generate_sequential_nodes_fold(
        prover,
        input.node_fold_proofs,
        &slot_indices,
        h,
        &format!("{e3_id}-nodesfold"),
        artifacts_dir,
    )?;

    let nodes_fold_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        CircuitName::NodesFold,
    )?;
    let c5_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        CircuitName::PkAggregation,
    )?;

    let party_id_fields: Vec<String> = input
        .party_ids
        .iter()
        .copied()
        .map(u64_to_field_hex)
        .collect();

    let witness = DkgAggregatorWitness {
        nodes_fold_vk: nodes_fold_vk.verification_key.clone(),
        nodes_fold_proof: proof_field_strings(&nodes_fold_proof)?,
        nodes_fold_public: proof_public_field_strings(&nodes_fold_proof)?,
        c5_vk: c5_vk.verification_key.clone(),
        c5_proof: proof_field_strings(input.c5_proof)?,
        c5_public: proof_public_field_strings(input.c5_proof)?,
        nodes_fold_key_hash: nodes_fold_vk.key_hash.clone(),
        c5_key_hash: c5_vk.key_hash.clone(),
        party_ids: party_id_fields,
    };

    let json =
        serde_json::to_value(&witness).map_err(|e| ZkError::SerializationError(e.to_string()))?;
    let input_map = inputs_json_to_input_map(&json)?;
    let compiled = load_compiled_circuit(
        prover,
        CircuitName::DkgAggregator,
        CircuitVariant::Default,
        artifacts_dir,
    )?;
    let w = WitnessGenerator::new().generate_witness(&compiled, input_map)?;
    prover.generate_proof_with_variant(
        CircuitName::DkgAggregator,
        &w,
        e3_id,
        CircuitVariant::Evm,
        artifacts_dir,
    )
}

/// One ciphertext index: C6 inners + C7 proof.
pub struct DecryptionAggregationJob<'a> {
    pub c6_inner_proofs: &'a [Proof],
    pub c6_slot_indices: &'a [u32],
    pub c7_proof: &'a Proof,
}

#[derive(Serialize)]
struct DecryptionAggregatorWitness {
    c6_fold_vk: Vec<String>,
    c6_fold_proof: Vec<String>,
    c6_fold_public: Vec<String>,
    c7_vk: Vec<String>,
    c7_proof: Vec<String>,
    c7_public: Vec<String>,
    c6_fold_key_hash: String,
    c7_key_hash: String,
}

/// Prove [`CircuitName::DecryptionAggregator`] for each job (C6 fold + C7), with
/// [`CircuitVariant::Evm`] for on-chain verification.
pub fn prove_decryption_aggregation_jobs(
    prover: &ZkProver,
    c6_total_slots: usize,
    jobs: &[DecryptionAggregationJob],
    e3_id: &str,
    artifacts_dir: &str,
) -> Result<Vec<Proof>, ZkError> {
    // VKs and the compiled circuit are job-independent: load once, reuse per ciphertext.
    let c6_fold_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        CircuitName::C6Fold,
    )?;
    let c7_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        CircuitName::DecryptedSharesAggregation,
    )?;
    let compiled = load_compiled_circuit(
        prover,
        CircuitName::DecryptionAggregator,
        CircuitVariant::Default,
        artifacts_dir,
    )?;

    let mut out = Vec::with_capacity(jobs.len());
    for (i, job) in jobs.iter().enumerate() {
        let c6_fold = generate_sequential_c6_fold(
            prover,
            job.c6_inner_proofs,
            job.c6_slot_indices,
            c6_total_slots,
            &format!("{e3_id}-c6fold-{i}"),
            artifacts_dir,
        )?;

        let witness = DecryptionAggregatorWitness {
            c6_fold_vk: c6_fold_vk.verification_key.clone(),
            c6_fold_proof: proof_field_strings(&c6_fold)?,
            c6_fold_public: proof_public_field_strings(&c6_fold)?,
            c7_vk: c7_vk.verification_key.clone(),
            c7_proof: proof_field_strings(job.c7_proof)?,
            c7_public: proof_public_field_strings(job.c7_proof)?,
            c6_fold_key_hash: c6_fold_vk.key_hash.clone(),
            c7_key_hash: c7_vk.key_hash.clone(),
        };

        let json = serde_json::to_value(&witness)
            .map_err(|e| ZkError::SerializationError(e.to_string()))?;
        let input_map = inputs_json_to_input_map(&json)?;
        let w = WitnessGenerator::new().generate_witness(&compiled, input_map)?;
        let proof = prover.generate_proof_with_variant(
            CircuitName::DecryptionAggregator,
            &w,
            &format!("{e3_id}-decagg-{i}"),
            CircuitVariant::Evm,
            artifacts_dir,
        )?;
        out.push(proof);
    }
    Ok(out)
}
