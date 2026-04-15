// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sequential C3 fold: each step verifies one inner `ShareEncryption` proof and the accumulator
//! (`c3_fold` non-ZK proof). The first step proves [`CircuitName::C3FoldKernel`] at runtime to obtain
//! a valid genesis `UltraHonkProof` (see `circuits/bin/recursive_aggregation/c3_fold_kernel`).
//!
//! Ciphernodes integrate via [`generate_sequential_c3_fold`] only: they supply the full list of C3
//! inner proofs and slot indices; per-step folding is not exposed outside this crate.

use crate::circuits::utils::{bytes_to_field_strings, inputs_json_to_input_map};
use crate::circuits::vk;
use crate::error::ZkError;
use crate::prover::ZkProver;
use crate::witness::{CompiledCircuit, WitnessGenerator};
use e3_events::{CircuitName, CircuitVariant, Proof};
use serde::Serialize;

/// Field count for `UltraHonkProof` (non-ZK) in `c3_fold` — from `nargo compile` ABI (`acc_proof`).
const C3_FOLD_ACC_NONZK_PROOF_FIELDS: usize = 457;

/// `total_slots` = N_PARTIES * L_THRESHOLD (one slot per party-modulus pair).
fn c3_fold_public_input_field_count(total_slots: usize) -> usize {
    4 + 3 * total_slots
}

fn zero_field_hex_strings(field_count: usize) -> Result<Vec<String>, ZkError> {
    let bytes = vec![0u8; field_count * 32];
    bytes_to_field_strings(&bytes)
}

/// Proves [`CircuitName::C3FoldKernel`] for the same `inner` / `total_slots` as the fold step.
///
/// Uses work dir `job_id` (caller should use a suffix of the fold `e3_id` so jobs stay distinct).
/// Removes that work dir after the proof is returned.
fn generate_c3_fold_kernel_genesis_proof(
    prover: &ZkProver,
    inner: &Proof,
    total_slots: usize,
    artifacts_dir: &str,
    job_id: &str,
) -> Result<Proof, ZkError> {
    let inner_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Recursive, artifacts_dir),
        CircuitName::ShareEncryption,
    )?;
    let kernel_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        CircuitName::C3FoldKernel,
    )?;
    let c3_public_inputs = share_encryption_inner_public_inputs(inner)?;
    let expected_acc_pub = c3_fold_public_input_field_count(total_slots);
    let acc_pi = zero_field_hex_strings(expected_acc_pub)?;
    let acc_pf = zero_field_hex_strings(C3_FOLD_ACC_NONZK_PROOF_FIELDS)?;

    let full_input = C3FoldStepInput {
        inner_vk: inner_vk.verification_key,
        inner_proof: bytes_to_field_strings(&inner.data)?,
        c3_public_inputs,
        acc_vk: kernel_vk.verification_key,
        acc_proof: acc_pf,
        acc_public_inputs: acc_pi,
        inner_key_hash: inner_vk.key_hash,
        acc_key_hash: kernel_vk.key_hash,
        is_first_step: true,
        slot_index: 0,
    };

    let circuit_path = prover
        .circuits_dir(CircuitVariant::Default, artifacts_dir)
        .join(CircuitName::C3FoldKernel.dir_path())
        .join(format!("{}.json", CircuitName::C3FoldKernel.as_str()));
    let compiled = CompiledCircuit::from_file(&circuit_path)?;

    let json = serde_json::to_value(&full_input)
        .map_err(|e| ZkError::SerializationError(e.to_string()))?;
    let input_map = inputs_json_to_input_map(&json)?;
    let witness_gen = WitnessGenerator::new();
    let witness = witness_gen.generate_witness(&compiled, input_map)?;

    let proof = prover.generate_recursive_aggregation_bin_proof(
        CircuitName::C3FoldKernel,
        &witness,
        job_id,
        artifacts_dir,
    )?;
    let _ = prover.cleanup(job_id);
    Ok(proof)
}

/// Extracts a single 32-byte field from a named proof signal, returning its hex string.
fn extract_single_field(proof: &Proof, kind: &str, name: &str) -> Result<String, ZkError> {
    let bytes = match kind {
        "input" => proof
            .extract_input(name)
            .ok_or_else(|| ZkError::InvalidInput(format!("C3 proof missing {name}")))?,
        "output" => proof
            .extract_output(name)
            .ok_or_else(|| ZkError::InvalidInput(format!("C3 proof missing {name}")))?,
        _ => unreachable!(),
    };
    let fields = bytes_to_field_strings(bytes.as_ref())?;
    if fields.len() != 1 {
        return Err(ZkError::InvalidInput(
            "C3 public signals must be three 32-byte fields (2 inputs + 1 output)".into(),
        ));
    }
    Ok(fields.into_iter().next().unwrap())
}

/// Inner C3 public transcript: two inputs + `ct_commitment` output.
fn share_encryption_inner_public_inputs(proof: &Proof) -> Result<[String; 3], ZkError> {
    if proof.circuit != CircuitName::ShareEncryption {
        return Err(ZkError::InvalidInput(format!(
            "expected ShareEncryption inner proof, got {}",
            proof.circuit
        )));
    }
    Ok([
        extract_single_field(proof, "input", "expected_pk_commitment")?,
        extract_single_field(proof, "input", "expected_message_commitment")?,
        extract_single_field(proof, "output", "ct_commitment")?,
    ])
}

#[derive(Serialize)]
struct C3FoldStepInput {
    inner_vk: Vec<String>,
    inner_proof: Vec<String>,
    c3_public_inputs: [String; 3],
    acc_vk: Vec<String>,
    acc_proof: Vec<String>,
    acc_public_inputs: Vec<String>,
    inner_key_hash: String,
    acc_key_hash: String,
    is_first_step: bool,
    slot_index: u32,
}

fn parse_c3_fold_public_field_strings(proof: &Proof) -> Result<Vec<String>, ZkError> {
    if proof.circuit != CircuitName::C3Fold {
        return Err(ZkError::InvalidInput(format!(
            "expected C3Fold proof, got {}",
            proof.circuit
        )));
    }
    let v = bytes_to_field_strings(proof.public_signals.as_ref())?;
    if v.len() < 4 || (v.len() - 4) % 3 != 0 {
        return Err(ZkError::InvalidInput(format!(
            "unexpected c3_fold public signal field count: {}",
            v.len()
        )));
    }
    Ok(v)
}

/// One sequential `c3_fold` step.
///
/// `prior_fold` is `None` on the first step and `Some` on all subsequent steps.
/// `total_slots` is `N_PARTIES * L_THRESHOLD` — one slot per (party, threshold-modulus) pair.
/// On the first step this sets the accumulator size; on subsequent steps it is cross-checked
/// against the slot count already encoded in `prior_fold`.
///
/// Used only by [`generate_sequential_c3_fold`]; callers should use that entry point.
fn generate_c3_fold_step(
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
        &prover.circuits_dir(CircuitVariant::Recursive, artifacts_dir),
        CircuitName::ShareEncryption,
    )?;
    let c3_public_inputs = share_encryption_inner_public_inputs(inner)?;

    let expected_acc_pub = c3_fold_public_input_field_count(total_slots);

    let (acc_vk_art, acc_proof, acc_public_inputs) = if is_first_step {
        let kernel_vk = vk::load_vk_artifacts(
            &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
            CircuitName::C3FoldKernel,
        )?;
        let kernel_job_id = format!("{e3_id}-c3fold-kernel");
        let kernel_proof = generate_c3_fold_kernel_genesis_proof(
            prover,
            inner,
            total_slots,
            artifacts_dir,
            &kernel_job_id,
        )?;
        let acc_pi = bytes_to_field_strings(kernel_proof.public_signals.as_ref())?;
        if acc_pi.len() != expected_acc_pub {
            return Err(ZkError::InvalidInput(format!(
                "c3_fold kernel proof public_inputs field count {} != expected {} (total_slots={})",
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
        // Parse once; derive slot count from field count to avoid a second parse.
        let acc_pi = parse_c3_fold_public_field_strings(p)?;
        let prior_slots = (acc_pi.len() - 4) / 3;
        if prior_slots == 0 {
            return Err(ZkError::InvalidInput(
                "c3_fold proof implies zero slots".into(),
            ));
        }
        if prior_slots != total_slots {
            return Err(ZkError::InvalidInput(format!(
                "prior c3_fold slot count {} != expected {}",
                prior_slots, total_slots
            )));
        }
        if acc_pi.len() != expected_acc_pub {
            return Err(ZkError::InvalidInput(format!(
                "prior c3_fold public field count {} != expected {} for total_slots={}",
                acc_pi.len(),
                expected_acc_pub,
                total_slots
            )));
        }
        (
            vk::load_vk_artifacts(
                &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
                CircuitName::C3Fold,
            )?,
            bytes_to_field_strings(&p.data)?,
            acc_pi,
        )
    };

    let full_input = C3FoldStepInput {
        inner_vk: inner_vk.verification_key,
        inner_proof: bytes_to_field_strings(&inner.data)?,
        c3_public_inputs,
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

/// Folds `inner_proofs` in order, one inner C3 proof per step — the integration surface for
/// ciphernodes (batch in, single `C3Fold` proof out).
///
/// `slot_indices[i]` is the `(party * L_THRESHOLD + modulus)` slot for `inner_proofs[i]`.
/// `total_slots` must equal `N_PARTIES * L_THRESHOLD` and determines the accumulator size.
pub fn generate_sequential_c3_fold(
    prover: &ZkProver,
    inner_proofs: &[Proof],
    slot_indices: &[u32],
    total_slots: usize,
    e3_id: &str,
    artifacts_dir: &str,
) -> Result<Proof, ZkError> {
    if inner_proofs.is_empty() {
        return Err(ZkError::InvalidInput(
            "generate_sequential_c3_fold: need at least one inner proof".into(),
        ));
    }
    if inner_proofs.len() != slot_indices.len() {
        return Err(ZkError::InvalidInput(
            "inner_proofs and slot_indices length mismatch".into(),
        ));
    }
    let mut acc: Option<Proof> = None;
    for (i, inner) in inner_proofs.iter().enumerate() {
        let out = generate_c3_fold_step(
            prover,
            inner,
            acc.as_ref(),
            slot_indices[i],
            total_slots,
            e3_id,
            artifacts_dir,
        )?;
        acc = Some(out);
    }
    Ok(acc.expect("non-empty loop"))
}
