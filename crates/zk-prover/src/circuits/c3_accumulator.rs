// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! C3 binary-fold tree: `c3_fold` (ZK inner → N-slot) and `c3_fold_merge` (non-ZK merge).

use super::utils::{bytes_to_field_strings, inputs_json_to_input_map};
use crate::circuits::vk;
use crate::error::ZkError;
use crate::prover::ZkProver;
use crate::witness::{CompiledCircuit, WitnessGenerator};
use e3_events::{CircuitName, CircuitVariant, Proof};
use serde::Serialize;

/// Inner C3 public transcript: `expected_pk`, `expected_message`, then `ct_commitment` (return).
fn share_encryption_inner_public_inputs(proof: &Proof) -> Result<[String; 3], ZkError> {
    if proof.circuit != CircuitName::ShareEncryption {
        return Err(ZkError::InvalidInput(format!(
            "expected ShareEncryption inner proof, got {}",
            proof.circuit
        )));
    }
    let pk = proof
        .extract_input("expected_pk_commitment")
        .ok_or_else(|| ZkError::InvalidInput("C3 proof missing expected_pk_commitment".into()))?;
    let msg = proof
        .extract_input("expected_message_commitment")
        .ok_or_else(|| {
            ZkError::InvalidInput("C3 proof missing expected_message_commitment".into())
        })?;
    let ct = proof
        .extract_output("ct_commitment")
        .ok_or_else(|| ZkError::InvalidInput("C3 proof missing ct_commitment output".into()))?;
    let p0 = bytes_to_field_strings(pk.as_ref())?;
    let p1 = bytes_to_field_strings(msg.as_ref())?;
    let p2 = bytes_to_field_strings(ct.as_ref())?;
    if p0.len() != 1 || p1.len() != 1 || p2.len() != 1 {
        return Err(ZkError::InvalidInput(
            "C3 public signals must be three 32-byte fields (2 inputs + 1 output)".into(),
        ));
    }
    Ok([p0[0].clone(), p1[0].clone(), p2[0].clone()])
}

/// Witness input for [`CircuitName::C3Fold`] (`circuits/bin/recursive_aggregation/c3_fold`).
#[derive(Serialize)]
struct C3FoldInput {
    vk: Vec<String>,
    proof1: Vec<String>,
    c3_public_inputs1: [String; 3],
    key_hash: String,
    slot_index1: u32,
    proof2: Vec<String>,
    c3_public_inputs2: [String; 3],
    slot_index2: u32,
    skip_second_proof: bool,
}

/// Witness input for [`CircuitName::C3FoldMerge`] (`circuits/bin/recursive_aggregation/c3_fold_merge`).
#[derive(Serialize)]
struct C3FoldMergeInput {
    vk1: Vec<String>,
    proof1: Vec<String>,
    pk_commits1: Vec<String>,
    msg_commits1: Vec<String>,
    key_hash1: String,
    vk2: Vec<String>,
    proof2: Vec<String>,
    pk_commits2: Vec<String>,
    msg_commits2: Vec<String>,
    key_hash2: String,
}

fn field_strings_from_c3_fold_public_signals(signals: &[u8]) -> Result<Vec<String>, ZkError> {
    let v = bytes_to_field_strings(signals)?;
    Ok(v)
}

/// Generates a `c3_fold` proof from one or two inner C3 (`ShareEncryption`, recursive) proofs.
///
/// When `skip_second_proof` is true (odd leaf in a pair), only `proof1` is verified on-chain;
/// `proof2` is still passed to the witness (use a duplicate of `proof1` to satisfy fixed-size ABI).
pub fn generate_c3_fold_proof(
    prover: &ZkProver,
    proof1: &Proof,
    slot_index1: u32,
    proof2: &Proof,
    slot_index2: u32,
    skip_second_proof: bool,
    e3_id: &str,
    artifacts_dir: &str,
) -> Result<Proof, ZkError> {
    let vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Recursive, artifacts_dir),
        CircuitName::ShareEncryption,
    )?;
    let c3_public_inputs1 = share_encryption_inner_public_inputs(proof1)?;
    let c3_public_inputs2 = share_encryption_inner_public_inputs(proof2)?;

    let full_input = C3FoldInput {
        vk: vk.verification_key,
        proof1: bytes_to_field_strings(&proof1.data)?,
        c3_public_inputs1,
        key_hash: vk.key_hash,
        slot_index1,
        proof2: bytes_to_field_strings(&proof2.data)?,
        c3_public_inputs2,
        slot_index2,
        skip_second_proof,
    };

    let circuit_path = prover
        .circuits_dir(CircuitVariant::Default, artifacts_dir)
        .join(CircuitName::C3Fold.dir_path())
        .join(format!("{}.json", CircuitName::C3Fold.as_str()));
    let compiled = CompiledCircuit::from_file(&circuit_path)?;

    let json = serde_json::to_value(&full_input)
        .map_err(|e| ZkError::SerializationError(e.to_string()))?;
    let input_map = inputs_json_to_input_map(&json)?;

    let witness_gen = WitnessGenerator::new();
    let witness = witness_gen.generate_witness(&compiled, input_map)?;

    prover.generate_recursive_aggregation_bin_proof(
        CircuitName::C3Fold,
        &witness,
        e3_id,
        artifacts_dir,
    )
}

/// Merges two N-slot `c3_fold` / `c3_fold_merge` proofs using [`CircuitName::C3FoldMerge`].
pub fn generate_c3_fold_merge_proof(
    prover: &ZkProver,
    n_slot_proof1: &Proof,
    n_slot_proof2: &Proof,
    e3_id: &str,
    artifacts_dir: &str,
) -> Result<Proof, ZkError> {
    if n_slot_proof1.circuit != CircuitName::C3Fold
        && n_slot_proof1.circuit != CircuitName::C3FoldMerge
    {
        return Err(ZkError::InvalidInput(format!(
            "merge left input must be C3Fold or C3FoldMerge, got {}",
            n_slot_proof1.circuit
        )));
    }
    if n_slot_proof2.circuit != CircuitName::C3Fold
        && n_slot_proof2.circuit != CircuitName::C3FoldMerge
    {
        return Err(ZkError::InvalidInput(format!(
            "merge right input must be C3Fold or C3FoldMerge, got {}",
            n_slot_proof2.circuit
        )));
    }

    let vk1 = vk::load_vk_for_fold_input(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        n_slot_proof1.circuit,
    )?;
    let vk2 = vk::load_vk_for_fold_input(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        n_slot_proof2.circuit,
    )?;

    let f1 = field_strings_from_c3_fold_public_signals(n_slot_proof1.public_signals.as_ref())?;
    let f2 = field_strings_from_c3_fold_public_signals(n_slot_proof2.public_signals.as_ref())?;

    if f1.len() != f2.len() || f1.len() % 2 != 0 {
        return Err(ZkError::InvalidInput(format!(
            "C3 N-slot public signals must be two equal-length halves (pk || msg), got {} and {}",
            f1.len(),
            f2.len()
        )));
    }
    let n = f1.len() / 2;

    let pk_commits1 = f1[..n].to_vec();
    let msg_commits1 = f1[n..].to_vec();
    let pk_commits2 = f2[..n].to_vec();
    let msg_commits2 = f2[n..].to_vec();

    let full_input = C3FoldMergeInput {
        vk1: vk1.verification_key,
        proof1: bytes_to_field_strings(&n_slot_proof1.data)?,
        pk_commits1,
        msg_commits1,
        key_hash1: vk1.key_hash,
        vk2: vk2.verification_key,
        proof2: bytes_to_field_strings(&n_slot_proof2.data)?,
        pk_commits2,
        msg_commits2,
        key_hash2: vk2.key_hash,
    };

    let circuit_path = prover
        .circuits_dir(CircuitVariant::Default, artifacts_dir)
        .join(CircuitName::C3FoldMerge.dir_path())
        .join(format!("{}.json", CircuitName::C3FoldMerge.as_str()));
    let compiled = CompiledCircuit::from_file(&circuit_path)?;

    let json = serde_json::to_value(&full_input)
        .map_err(|e| ZkError::SerializationError(e.to_string()))?;
    let input_map = inputs_json_to_input_map(&json)?;

    let witness_gen = WitnessGenerator::new();
    let witness = witness_gen.generate_witness(&compiled, input_map)?;

    prover.generate_recursive_aggregation_bin_proof(
        CircuitName::C3FoldMerge,
        &witness,
        e3_id,
        artifacts_dir,
    )
}
