// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Shared helpers for the sequential `c3_fold` / `c6_fold` / `nodes_fold` accumulators and
//! the single-shot `node_dkg_fold` builder.

use crate::circuits::utils::bytes_to_field_strings;
use crate::error::ZkError;
use e3_events::{CircuitName, Proof};

/// Field count for `UltraHonkProof` (non-ZK) used as the `acc_proof` parameter in every
/// recursive fold step. Sourced from `nargo compile` ABI.
pub const ACC_NONZK_PROOF_FIELDS: usize = 457;

/// Vector of `field_count` zero-encoded 32-byte hex field strings for the genesis accumulator.
pub fn zero_field_hex_strings(field_count: usize) -> Result<Vec<String>, ZkError> {
    let bytes = vec![0u8; field_count * 32];
    bytes_to_field_strings(&bytes)
}

/// Encode a `u64` as a 32-byte field hex string (big-endian, left-padded).
pub fn u64_to_field_hex(value: u64) -> String {
    let mut bytes = [0u8; 32];
    bytes[24..].copy_from_slice(&value.to_be_bytes());
    format!("0x{}", hex::encode(bytes))
}

/// Extract a single 32-byte public input/output by name. `kind` must be `"input"` or `"output"`.
pub fn extract_single_field(
    proof: &Proof,
    kind: &str,
    name: &str,
    context: &str,
) -> Result<String, ZkError> {
    let bytes = match kind {
        "input" => proof
            .extract_input(name)
            .ok_or_else(|| ZkError::InvalidInput(format!("{context}: missing input {name}")))?,
        "output" => proof
            .extract_output(name)
            .ok_or_else(|| ZkError::InvalidInput(format!("{context}: missing output {name}")))?,
        _ => {
            return Err(ZkError::InvalidInput(format!(
                "extract_single_field: kind must be input or output, got {kind}"
            )));
        }
    };
    let fields = bytes_to_field_strings(bytes.as_ref())?;
    if fields.len() != 1 {
        return Err(ZkError::InvalidInput(format!(
            "{context}: field {name} encoded as {} fields, expected 1",
            fields.len()
        )));
    }
    Ok(fields.into_iter().next().expect("len == 1 verified above"))
}

/// Parse and validate fold-accumulator public-signal field strings against
/// `(prefix_len, slot_width)`. Returns the flattened field-string vector.
pub fn parse_acc_public_field_strings(
    proof: &Proof,
    expected_circuit: CircuitName,
    prefix_len: usize,
    slot_width: usize,
) -> Result<Vec<String>, ZkError> {
    if proof.circuit != expected_circuit {
        return Err(ZkError::InvalidInput(format!(
            "expected {expected_circuit} proof, got {}",
            proof.circuit
        )));
    }
    let v = bytes_to_field_strings(proof.public_signals.as_ref())?;
    if v.len() < prefix_len || (v.len() - prefix_len) % slot_width != 0 {
        return Err(ZkError::InvalidInput(format!(
            "unexpected {expected_circuit} public signal field count: {} (prefix={prefix_len}, slot_width={slot_width})",
            v.len()
        )));
    }
    Ok(v)
}

/// Sequential fold driver: applies `step(inner, prior_acc, slot_index)` per inner proof,
/// threading each step's output as the next step's accumulator.
pub fn sequential_fold<F>(
    label: &str,
    inner_proofs: &[Proof],
    slot_indices: &[u32],
    mut step: F,
) -> Result<Proof, ZkError>
where
    F: FnMut(&Proof, Option<&Proof>, u32) -> Result<Proof, ZkError>,
{
    if inner_proofs.is_empty() {
        return Err(ZkError::InvalidInput(format!(
            "{label}: need at least one inner proof"
        )));
    }
    if inner_proofs.len() != slot_indices.len() {
        return Err(ZkError::InvalidInput(format!(
            "{label}: inner_proofs and slot_indices length mismatch ({} vs {})",
            inner_proofs.len(),
            slot_indices.len()
        )));
    }
    let mut acc: Option<Proof> = None;
    for (i, inner) in inner_proofs.iter().enumerate() {
        acc = Some(step(inner, acc.as_ref(), slot_indices[i])?);
    }
    Ok(acc.expect("loop body executed at least once on non-empty input"))
}
