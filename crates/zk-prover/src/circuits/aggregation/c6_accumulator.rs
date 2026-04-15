// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sequential C6 fold: each step verifies one inner `ThresholdShareDecryption` proof and the
//! accumulator (`c6_fold` non-ZK proof). The first step proves [`CircuitName::C6FoldKernel`] at
//! runtime to obtain a valid genesis `UltraHonkProof` (see `circuits/bin/recursive_aggregation/c6_fold_kernel`).

use crate::circuits::utils::{bytes_to_field_strings, inputs_json_to_input_map};
use crate::circuits::vk;
use crate::error::ZkError;
use crate::prover::ZkProver;
use crate::witness::{CompiledCircuit, WitnessGenerator};
use e3_events::{CircuitName, CircuitVariant, Proof};
use serde::Serialize;

/// Field count for `UltraHonkProof` (non-ZK) in `c6_fold` — from `nargo compile` ABI (`acc_proof`).
const C6_FOLD_ACC_NONZK_PROOF_FIELDS: usize = 457;

/// `total_slots` = `T + 1` (one slot per party index in the C6 leaf layout).
fn c6_fold_public_input_field_count(total_slots: usize) -> usize {
    4 + 4 * total_slots
}

fn zero_field_hex_strings(field_count: usize) -> Result<Vec<String>, ZkError> {
    let bytes = vec![0u8; field_count * 32];
    bytes_to_field_strings(&bytes)
}

fn generate_c6_fold_kernel_genesis_proof(
    prover: &ZkProver,
    inner: &Proof,
    total_slots: usize,
    artifacts_dir: &str,
    job_id: &str,
) -> Result<Proof, ZkError> {
    let inner_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Recursive, artifacts_dir),
        CircuitName::ThresholdShareDecryption,
    )?;
    let kernel_vk = vk::load_vk_artifacts(
        &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
        CircuitName::C6FoldKernel,
    )?;
    let c6_public_inputs = threshold_share_decryption_inner_public_inputs(inner)?;
    let expected_acc_pub = c6_fold_public_input_field_count(total_slots);
    let acc_pi = zero_field_hex_strings(expected_acc_pub)?;
    let acc_pf = zero_field_hex_strings(C6_FOLD_ACC_NONZK_PROOF_FIELDS)?;

    let full_input = C6FoldStepInput {
        inner_vk: inner_vk.verification_key,
        inner_proof: bytes_to_field_strings(&inner.data)?,
        c6_public_inputs,
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
        .join(CircuitName::C6FoldKernel.dir_path())
        .join(format!("{}.json", CircuitName::C6FoldKernel.as_str()));
    let compiled = CompiledCircuit::from_file(&circuit_path)?;

    let json = serde_json::to_value(&full_input)
        .map_err(|e| ZkError::SerializationError(e.to_string()))?;
    let input_map = inputs_json_to_input_map(&json)?;
    let witness_gen = WitnessGenerator::new();
    let witness = witness_gen.generate_witness(&compiled, input_map)?;

    let proof = prover.generate_recursive_aggregation_bin_proof(
        CircuitName::C6FoldKernel,
        &witness,
        job_id,
        artifacts_dir,
    )?;
    let _ = prover.cleanup(job_id);
    Ok(proof)
}

fn extract_single_field(proof: &Proof, kind: &str, name: &str) -> Result<String, ZkError> {
    let bytes = match kind {
        "input" => proof
            .extract_input(name)
            .ok_or_else(|| ZkError::InvalidInput(format!("C6 inner proof missing {name}")))?,
        "output" => proof
            .extract_output(name)
            .ok_or_else(|| ZkError::InvalidInput(format!("C6 inner proof missing {name}")))?,
        _ => unreachable!(),
    };
    let fields = bytes_to_field_strings(bytes.as_ref())?;
    if fields.len() != 1 {
        return Err(ZkError::InvalidInput(
            "C6 public signals must be four 32-byte fields (3 inputs + 1 output)".into(),
        ));
    }
    Ok(fields.into_iter().next().unwrap())
}

fn threshold_share_decryption_inner_public_inputs(proof: &Proof) -> Result<[String; 4], ZkError> {
    if proof.circuit != CircuitName::ThresholdShareDecryption {
        return Err(ZkError::InvalidInput(format!(
            "expected ThresholdShareDecryption inner proof, got {}",
            proof.circuit
        )));
    }
    Ok([
        extract_single_field(proof, "input", "expected_sk_commitment")?,
        extract_single_field(proof, "input", "expected_e_sm_commitment")?,
        extract_single_field(proof, "input", "ct_commitment")?,
        extract_single_field(proof, "output", "d_commitment")?,
    ])
}

#[derive(Serialize)]
struct C6FoldStepInput {
    inner_vk: Vec<String>,
    inner_proof: Vec<String>,
    c6_public_inputs: [String; 4],
    acc_vk: Vec<String>,
    acc_proof: Vec<String>,
    acc_public_inputs: Vec<String>,
    inner_key_hash: String,
    acc_key_hash: String,
    is_first_step: bool,
    slot_index: u32,
}

fn parse_c6_fold_public_field_strings(proof: &Proof) -> Result<Vec<String>, ZkError> {
    if proof.circuit != CircuitName::C6Fold {
        return Err(ZkError::InvalidInput(format!(
            "expected C6Fold proof, got {}",
            proof.circuit
        )));
    }
    let v = bytes_to_field_strings(proof.public_signals.as_ref())?;
    if v.len() < 4 || (v.len() - 4) % 4 != 0 {
        return Err(ZkError::InvalidInput(format!(
            "unexpected c6_fold public signal field count: {}",
            v.len()
        )));
    }
    Ok(v)
}

fn generate_c6_fold_step(
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
        CircuitName::ThresholdShareDecryption,
    )?;
    let c6_public_inputs = threshold_share_decryption_inner_public_inputs(inner)?;

    let expected_acc_pub = c6_fold_public_input_field_count(total_slots);

    let (acc_vk_art, acc_proof, acc_public_inputs) = if is_first_step {
        let kernel_vk = vk::load_vk_artifacts(
            &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
            CircuitName::C6FoldKernel,
        )?;
        let kernel_job_id = format!("{e3_id}-c6fold-kernel");
        let kernel_proof = generate_c6_fold_kernel_genesis_proof(
            prover,
            inner,
            total_slots,
            artifacts_dir,
            &kernel_job_id,
        )?;
        let acc_pi = bytes_to_field_strings(kernel_proof.public_signals.as_ref())?;
        if acc_pi.len() != expected_acc_pub {
            return Err(ZkError::InvalidInput(format!(
                "c6_fold kernel proof public_inputs field count {} != expected {} (total_slots={})",
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
        let acc_pi = parse_c6_fold_public_field_strings(p)?;
        let prior_slots = (acc_pi.len() - 4) / 4;
        if prior_slots == 0 {
            return Err(ZkError::InvalidInput(
                "c6_fold proof implies zero slots".into(),
            ));
        }
        if prior_slots != total_slots {
            return Err(ZkError::InvalidInput(format!(
                "prior c6_fold slot count {} != expected {}",
                prior_slots, total_slots
            )));
        }
        if acc_pi.len() != expected_acc_pub {
            return Err(ZkError::InvalidInput(format!(
                "prior c6_fold public field count {} != expected {} for total_slots={}",
                acc_pi.len(),
                expected_acc_pub,
                total_slots
            )));
        }
        (
            vk::load_vk_artifacts(
                &prover.circuits_dir(CircuitVariant::Default, artifacts_dir),
                CircuitName::C6Fold,
            )?,
            bytes_to_field_strings(&p.data)?,
            acc_pi,
        )
    };

    let full_input = C6FoldStepInput {
        inner_vk: inner_vk.verification_key,
        inner_proof: bytes_to_field_strings(&inner.data)?,
        c6_public_inputs,
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
        .join(CircuitName::C6Fold.dir_path())
        .join(format!("{}.json", CircuitName::C6Fold.as_str()));
    let compiled = CompiledCircuit::from_file(&circuit_path)?;

    let json = serde_json::to_value(&full_input)
        .map_err(|e| ZkError::SerializationError(e.to_string()))?;
    let input_map = inputs_json_to_input_map(&json)?;

    let witness_gen = WitnessGenerator::new();
    let witness = witness_gen.generate_witness(&compiled, input_map)?;

    prover.generate_recursive_aggregation_bin_proof(
        CircuitName::C6Fold,
        &witness,
        e3_id,
        artifacts_dir,
    )
}

/// Folds `inner_proofs` in order, one inner C6 proof per step — the integration surface for
/// ciphernodes (batch in, single `C6Fold` proof out).
///
/// `slot_indices[i]` is the party slot index for `inner_proofs[i]`.
/// `total_slots` must equal `T + 1` and determines the accumulator width.
pub fn generate_sequential_c6_fold(
    prover: &ZkProver,
    inner_proofs: &[Proof],
    slot_indices: &[u32],
    total_slots: usize,
    e3_id: &str,
    artifacts_dir: &str,
) -> Result<Proof, ZkError> {
    if inner_proofs.is_empty() {
        return Err(ZkError::InvalidInput(
            "generate_sequential_c6_fold: need at least one inner proof".into(),
        ));
    }
    if inner_proofs.len() != slot_indices.len() {
        return Err(ZkError::InvalidInput(
            "inner_proofs and slot_indices length mismatch".into(),
        ));
    }
    let mut acc: Option<Proof> = None;
    for (i, inner) in inner_proofs.iter().enumerate() {
        let out = generate_c6_fold_step(
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
