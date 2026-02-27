// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Loads verification key and hash for inner circuits (wrapper proof aggregation).
//! Reads `.vk_recursive` and `.vk_recursive_hash` (poseidon2/noir-recursive-no-zk format).

use super::utils::bytes_to_field_strings;
use crate::error::ZkError;
use e3_events::CircuitName;
use std::fs;
use std::path::Path;

/// Inner circuit VK artifacts for recursive verification.
pub struct VkArtifacts {
    pub verification_key: Vec<String>,
    pub key_hash: String,
}

fn load_vk_from_dir(circuit_dir: &Path, circuit_name: &str) -> Result<VkArtifacts, ZkError> {
    let vk_path = circuit_dir.join(format!("{}.vk_recursive", circuit_name));
    let vk_hash_path = circuit_dir.join(format!("{}.vk_recursive_hash", circuit_name));

    let vk_bytes = fs::read(&vk_path)
        .map_err(|e| ZkError::CircuitNotFound(format!("{}: {}", vk_path.display(), e)))?;
    let vk_hash_bytes = fs::read(&vk_hash_path)
        .map_err(|e| ZkError::CircuitNotFound(format!("{}: {}", vk_hash_path.display(), e)))?;

    if vk_hash_bytes.len() != 32 {
        return Err(ZkError::InvalidInput(format!(
            "{}: expected 32 bytes, got {}",
            vk_hash_path.display(),
            vk_hash_bytes.len()
        )));
    }

    let verification_key = bytes_to_field_strings(&vk_bytes)?;
    let key_hash = format!("0x{}", hex::encode(&vk_hash_bytes));

    Ok(VkArtifacts {
        verification_key,
        key_hash,
    })
}

/// Loads recursive VK artifacts from the wrapper circuit dir.
/// Use when folding wrapper proofs (verifier needs the wrapper's VK).
pub fn load_wrapper_vk_artifacts(
    circuits_dir: &Path,
    circuit: CircuitName,
) -> Result<VkArtifacts, ZkError> {
    let circuit_dir = circuits_dir.join(circuit.wrapper_dir_path());
    load_vk_from_dir(&circuit_dir, circuit.as_str())
}

/// Loads recursive VK artifacts from `.vk_recursive` and `.vk_recursive_hash`.
/// Uses poseidon2 format (noir-recursive-no-zk) to match bb_proof_verification.
pub fn load_vk_artifacts(
    circuits_dir: &Path,
    circuit: CircuitName,
) -> Result<VkArtifacts, ZkError> {
    let circuit_dir = circuits_dir.join(circuit.dir_path());
    load_vk_from_dir(&circuit_dir, circuit.as_str())
}

/// VK path by circuit type: Fold uses dir_path, wrappers use wrapper_dir_path.
pub fn load_vk_for_fold_input(
    circuits_dir: &Path,
    circuit: CircuitName,
) -> Result<VkArtifacts, ZkError> {
    if circuit == CircuitName::Fold {
        load_vk_artifacts(circuits_dir, circuit)
    } else {
        load_wrapper_vk_artifacts(circuits_dir, circuit)
    }
}
