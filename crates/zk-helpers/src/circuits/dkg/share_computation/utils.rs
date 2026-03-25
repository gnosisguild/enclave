// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Shared utilities for the share-computation circuit: parity matrix helpers and build-time
//! helpers that combine recursive `vk_hash` blobs like the final Noir `share_computation` circuit.

use crate::compute_vk_hash;
use crate::utils::bigint_to_field;
use crate::CircuitsErrors;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use e3_parity_matrix::build_generator_matrix;
use e3_parity_matrix::{null_space, ParityMatrix, ParityMatrixConfig};
use num_bigint::{BigInt, BigUint};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Computes the parity check matrix (null space of the Reed–Solomon generator) per modulus.
///
/// Returns one `ParityMatrix` per modulus in `moduli`, each of shape `[n_parties - threshold][n_parties + 1]`.
pub fn compute_parity_matrix(
    moduli: &[u64],
    n_parties: usize,
    threshold: usize,
) -> Result<Vec<ParityMatrix>, String> {
    let mut parity_matrix = Vec::with_capacity(moduli.len());
    for &qi in moduli {
        let q = BigUint::from(qi);
        let g = build_generator_matrix(&ParityMatrixConfig {
            q: q.clone(),
            t: threshold,
            n: n_parties,
        })
        .map_err(|e| format!("Failed to build generator matrix: {:?}", e))?;
        let h = null_space(&g, &q).map_err(|e| format!("Failed to compute null space: {:?}", e))?;
        parity_matrix.push(h);
    }
    Ok(parity_matrix)
}

/// Builds the PARITY_MATRIX constant string for Noir (one matrix per modulus via null_space).
pub fn parity_matrix_constant_string(
    threshold_params: &std::sync::Arc<fhe::bfv::BfvParameters>,
    n_parties: usize,
    threshold: usize,
) -> Result<String, CircuitsErrors> {
    let parity_matrix = compute_parity_matrix(threshold_params.moduli(), n_parties, threshold)
        .map_err(|e| CircuitsErrors::Sample(e))?;

    let parity_matrix_strings: Vec<String> = parity_matrix
        .iter()
        .map(|h_mod| {
            let modulus_rows: Vec<String> = h_mod
                .data()
                .iter()
                .map(|row| {
                    let row_values: Vec<String> = row
                        .iter()
                        .map(|val| {
                            let bigint_val = BigInt::from(val.clone());
                            let field_val = bigint_to_field(&bigint_val);
                            field_val.to_string()
                        })
                        .collect();
                    format!("[{}]", row_values.join(", "))
                })
                .collect();
            format!("[\n        {}]", modulus_rows.join(",\n        "))
        })
        .collect();

    Ok(format!(
        "pub global PARITY_MATRIX: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L_THRESHOLD] = [\n    {}];",
        parity_matrix_strings.join(",\n    ")
    ))
}

/// Root directory that contains `bin/dkg/` (i.e. the `circuits` folder in the Enclave repo).
pub fn resolve_enclave_circuits_root() -> Option<PathBuf> {
    if let Ok(root) = env::var("ENCLAVE_CIRCUITS_ROOT") {
        let p = PathBuf::from(root);
        if p.join("bin/dkg/target").is_dir() {
            return Some(p);
        }
    }
    let mut dir = env::current_dir().ok()?;
    for _ in 0..10 {
        let cand = dir.join("circuits").join("bin").join("dkg").join("target");
        if cand.is_dir() {
            return Some(dir.join("circuits"));
        }
        dir = dir.parent()?.to_path_buf();
    }
    None
}

fn fr_from_vk_hash_file(path: &Path) -> Result<Fr, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() != 32 {
        return Err(format!(
            "{}: expected 32-byte vk_hash, got {}",
            path.display(),
            bytes.len()
        ));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(Fr::from_be_bytes_mod_order(&arr))
}

fn field_to_noir_hex(fr: Fr) -> String {
    let repr = fr.into_bigint().to_bytes_be();
    let mut out = [0u8; 32];
    let start = 32usize.saturating_sub(repr.len());
    out[start..].copy_from_slice(&repr);
    format!("0x{}", hex::encode(out))
}

/// `dkg_target` is `.../circuits/bin/dkg/target` (Nargo target dir). After `pnpm build:circuits` /
/// `build-circuits.ts`, noir-recursive-no-zk hashes are named `{package}.vk_recursive_hash`.
pub fn share_computation_expected_vk_hash_hex_literals(
    dkg_target: &Path,
) -> Result<(String, String), String> {
    let sk_base =
        fr_from_vk_hash_file(&dkg_target.join("sk_share_computation_base.vk_recursive_hash"))?;
    let esm_base =
        fr_from_vk_hash_file(&dkg_target.join("e_sm_share_computation_base.vk_recursive_hash"))?;
    let chunk =
        fr_from_vk_hash_file(&dkg_target.join("share_computation_chunk.vk_recursive_hash"))?;
    let batch =
        fr_from_vk_hash_file(&dkg_target.join("share_computation_chunk_batch.vk_recursive_hash"))?;
    let sk_chain = compute_vk_hash(vec![sk_base, chunk, batch]);
    let esm_chain = compute_vk_hash(vec![esm_base, chunk, batch]);
    Ok((field_to_noir_hex(sk_chain), field_to_noir_hex(esm_chain)))
}
