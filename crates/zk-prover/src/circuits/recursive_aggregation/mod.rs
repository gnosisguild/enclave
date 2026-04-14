// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Recursive aggregation helpers for Noir `recursive_aggregation/*` bins.
//!
//! Generic per-circuit wrappers and the old two-proof `fold` binary were removed in favor of
//! ad-hoc fold circuits (`c2ab_fold`, `c6_fold`, `nodes_fold`, `node_fold`, `dkg_aggregator`, etc.). Until the prover wires
//! those pipelines end-to-end, `generate_wrapper_proof` returns the inner proof unchanged and
//! `generate_fold_proof` returns an error.

use crate::error::ZkError;
use e3_events::Proof;

/// Legacy hook: recursive wrapper Noir programs under `recursive_aggregation/wrapper/` were
/// deleted. Callers should migrate to the ad-hoc fold binaries; meanwhile the inner proof is
/// returned unchanged so downstream types still receive a `Proof` bundle.
pub fn generate_wrapper_proof(
    _prover: &crate::prover::ZkProver,
    proof: &Proof,
    _e3_id: &str,
    _artifacts_dir: &str,
) -> Result<Proof, ZkError> {
    Ok(proof.clone())
}

/// Legacy two-proof fold (`recursive_aggregation/fold`) was removed; use `dkg_aggregator` /
/// `decryption_aggregator` instead.
pub fn generate_fold_proof(
    _prover: &crate::prover::ZkProver,
    _proof1: &Proof,
    _proof2: &Proof,
    _e3_id: &str,
    _target_evm: bool,
    _artifacts_dir: &str,
) -> Result<Proof, ZkError> {
    Err(ZkError::InvalidInput(
        "recursive two-proof fold removed; use ad-hoc dkg_aggregator / decryption_aggregator circuits"
            .to_string(),
    ))
}
