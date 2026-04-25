// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sequential [`CircuitName::NodesFold`]: each step verifies one inner [`CircuitName::NodeFold`]
//! proof and the previous accumulator (`nodes_fold` non-ZK proof). The first step proves
//! [`CircuitName::NodesFoldKernel`] at runtime to obtain a valid genesis `UltraHonkProof` (see
//! `circuits/bin/recursive_aggregation/nodes_fold_kernel`).

use crate::circuits::aggregation::helpers::{
    ACC_NONZK_PROOF_FIELDS, parse_acc_public_field_strings, sequential_fold,
    zero_field_hex_strings,
};
use crate::circuits::utils::{bytes_to_field_strings, inputs_json_to_input_map};
use crate::circuits::vk;
use crate::error::ZkError;
use crate::prover::ZkProver;
use crate::witness::{CompiledCircuit, WitnessGenerator};
use e3_events::{CircuitName, CircuitVariant, Proof};
use serde::Serialize;

/// Public-signal layout of `nodes_fold`: 4-field prefix, then `node_fold_fields`-wide tail.
const NODES_FOLD_PREFIX_LEN: usize = 4;

fn node_fold_statement_field_count(proof: &Proof) -> Result<usize, ZkError> {
    if proof.circuit != CircuitName::NodeFold {
        return Err(ZkError::InvalidInput(format!(
            "expected NodeFold inner proof, got {}",
            proof.circuit
        )));
    }
    let v = bytes_to_field_strings(proof.public_signals.as_ref())?;
    if v.is_empty() {
        return Err(ZkError::InvalidInput(
            "NodeFold proof has empty public_signals".into(),
        ));
    }
    Ok(v.len())
}

fn nodes_fold_acc_public_len(node_fold_fields: usize, total_slots: usize) -> usize {
    4 + total_slots * node_fold_fields
}

#[derive(Serialize)]
struct NodesFoldStepInput {
    inner_vk: Vec<String>,
    inner_proof: Vec<String>,
    node_fold_public_inputs: Vec<String>,
    acc_vk: Vec<String>,
    acc_proof: Vec<String>,
    acc_public_inputs: Vec<String>,
    inner_key_hash: String,
    acc_key_hash: String,
    is_first_step: bool,
    slot_index: u32,
}

/// Proves [`CircuitName::NodesFoldKernel`] for the same `inner` / `total_slots` / `slot_index` as the
/// fold step.
fn generate_nodes_fold_kernel_genesis_proof(
    prover: &ZkProver,
    inner: &Proof,
    total_slots: usize,
    slot_index: u32,
    artifacts_dir: &str,
    job_id: &str,
) -> Result<Proof, ZkError> {
    let inner_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        CircuitName::NodeFold,
    )?;
    let kernel_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        CircuitName::NodesFoldKernel,
    )?;
    let nf_fields = node_fold_statement_field_count(inner)?;
    let node_fold_public_inputs = bytes_to_field_strings(inner.public_signals.as_ref())?;
    if node_fold_public_inputs.len() != nf_fields {
        return Err(ZkError::InvalidInput(
            "NodeFold public field length mismatch".into(),
        ));
    }
    let expected_acc_pub = nodes_fold_acc_public_len(nf_fields, total_slots);
    let acc_pi = zero_field_hex_strings(expected_acc_pub)?;
    let acc_pf = zero_field_hex_strings(ACC_NONZK_PROOF_FIELDS)?;

    let full_input = NodesFoldStepInput {
        inner_vk: inner_vk.verification_key,
        inner_proof: bytes_to_field_strings(&inner.data)?,
        node_fold_public_inputs,
        acc_vk: kernel_vk.verification_key,
        acc_proof: acc_pf,
        acc_public_inputs: acc_pi,
        inner_key_hash: inner_vk.key_hash,
        acc_key_hash: kernel_vk.key_hash,
        is_first_step: true,
        slot_index,
    };

    let circuit_path = prover
        .circuits_dir(CircuitVariant::Default, artifacts_dir)
        .join(CircuitName::NodesFoldKernel.dir_path())
        .join(format!("{}.json", CircuitName::NodesFoldKernel.as_str()));
    let compiled = CompiledCircuit::from_file(&circuit_path)?;

    let json = serde_json::to_value(&full_input)
        .map_err(|e| ZkError::SerializationError(e.to_string()))?;
    let input_map = inputs_json_to_input_map(&json)?;
    let witness_gen = WitnessGenerator::new();
    let witness = witness_gen.generate_witness(&compiled, input_map)?;

    let proof = prover.generate_recursive_aggregation_bin_proof(
        CircuitName::NodesFoldKernel,
        &witness,
        job_id,
        artifacts_dir,
    )?;
    let _ = prover.cleanup(job_id);
    Ok(proof)
}

fn parse_nodes_fold_public_field_strings(proof: &Proof) -> Result<Vec<String>, ZkError> {
    // `slot_width` of 1 here only enforces the 4-field prefix; per-step length is then
    // cross-checked against `expected_acc_pub` derived from the actual `node_fold_fields`.
    parse_acc_public_field_strings(proof, CircuitName::NodesFold, NODES_FOLD_PREFIX_LEN, 1)
}

fn generate_nodes_fold_step(
    prover: &ZkProver,
    inner: &Proof,
    prior_fold: Option<&Proof>,
    slot_index: u32,
    total_slots: usize,
    e3_id: &str,
    artifacts_dir: &str,
) -> Result<Proof, ZkError> {
    let is_first_step = prior_fold.is_none();

    let inner_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        CircuitName::NodeFold,
    )?;
    let nf_fields = node_fold_statement_field_count(inner)?;
    let node_fold_public_inputs = bytes_to_field_strings(inner.public_signals.as_ref())?;
    if node_fold_public_inputs.len() != nf_fields {
        return Err(ZkError::InvalidInput(
            "NodeFold public field length mismatch".into(),
        ));
    }

    let expected_acc_pub = nodes_fold_acc_public_len(nf_fields, total_slots);

    let (acc_vk_art, acc_proof, acc_public_inputs) = if is_first_step {
        let kernel_vk = vk::load_vk_artifacts(
            &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
            CircuitName::NodesFoldKernel,
        )?;
        let kernel_job_id = format!("{e3_id}-nodesfold-kernel");
        let kernel_proof = generate_nodes_fold_kernel_genesis_proof(
            prover,
            inner,
            total_slots,
            slot_index,
            artifacts_dir,
            &kernel_job_id,
        )?;
        let acc_pi = bytes_to_field_strings(kernel_proof.public_signals.as_ref())?;
        if acc_pi.len() != expected_acc_pub {
            return Err(ZkError::InvalidInput(format!(
                "nodes_fold kernel proof public_inputs field count {} != expected {} (total_slots={})",
                acc_pi.len(),
                expected_acc_pub,
                total_slots
            )));
        }
        (
            kernel_vk,
            bytes_to_field_strings(&kernel_proof.data)?,
            acc_pi,
        )
    } else {
        let p = prior_fold.expect("prior_fold required when is_first_step is false");
        let acc_pi = parse_nodes_fold_public_field_strings(p)?;
        if acc_pi.len() != expected_acc_pub {
            return Err(ZkError::InvalidInput(format!(
                "prior nodes_fold public field count {} != expected {} (total_slots={})",
                acc_pi.len(),
                expected_acc_pub,
                total_slots
            )));
        }
        (
            vk::load_vk_artifacts(
                &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
                CircuitName::NodesFold,
            )?,
            bytes_to_field_strings(&p.data)?,
            acc_pi,
        )
    };

    let full_input = NodesFoldStepInput {
        inner_vk: inner_vk.verification_key,
        inner_proof: bytes_to_field_strings(&inner.data)?,
        node_fold_public_inputs,
        acc_vk: acc_vk_art.verification_key,
        acc_proof,
        acc_public_inputs,
        inner_key_hash: inner_vk.key_hash,
        acc_key_hash: acc_vk_art.key_hash,
        is_first_step,
        slot_index,
    };

    let circuit_path = prover
        .circuits_dir(CircuitVariant::Default, artifacts_dir)
        .join(CircuitName::NodesFold.dir_path())
        .join(format!("{}.json", CircuitName::NodesFold.as_str()));
    let compiled = CompiledCircuit::from_file(&circuit_path)?;

    let json = serde_json::to_value(&full_input)
        .map_err(|e| ZkError::SerializationError(e.to_string()))?;
    let input_map = inputs_json_to_input_map(&json)?;

    let witness_gen = WitnessGenerator::new();
    let witness = witness_gen.generate_witness(&compiled, input_map)?;

    prover.generate_recursive_aggregation_bin_proof(
        CircuitName::NodesFold,
        &witness,
        e3_id,
        artifacts_dir,
    )
}

/// Folds `inner_proofs` (one [`CircuitName::NodeFold`] per honest party) into a single
/// [`CircuitName::NodesFold`] proof for [`CircuitName::DkgAggregator`].
///
/// `slot_indices[i]` is the honest-slot index for `inner_proofs[i]` (must be `< total_slots`).
/// `total_slots` is `H` (honest committee size).
pub fn generate_sequential_nodes_fold(
    prover: &ZkProver,
    inner_proofs: &[Proof],
    slot_indices: &[u32],
    total_slots: usize,
    e3_id: &str,
    artifacts_dir: &str,
) -> Result<Proof, ZkError> {
    sequential_fold(
        "generate_sequential_nodes_fold",
        inner_proofs,
        slot_indices,
        |inner, prior, slot| {
            generate_nodes_fold_step(
                prover,
                inner,
                prior,
                slot,
                total_slots,
                e3_id,
                artifacts_dir,
            )
        },
    )
}
