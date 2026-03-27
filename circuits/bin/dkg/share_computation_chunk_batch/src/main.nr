// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Level 1: chunk_batch wrapper (verifies base + per-batch chunk proofs).

use bb_proof_verification::{UltraHonkVerificationKey, UltraHonkZKProof, verify_honk_proof};
use lib::configs::default::dkg::{
    L_THRESHOLD, N, SHARE_COMPUTATION_CHUNK_SIZE,
    SHARE_COMPUTATION_CHUNKS_PER_BATCH as CHUNKS_PER_BATCH,
};
use lib::configs::default::N_PARTIES;
use lib::math::commitments::compute_recursive_aggregation_commitment;

pub global BASE_PUBLIC_INPUTS: u32 =
    1 + (N * L_THRESHOLD * (N_PARTIES + 1)) + (N_PARTIES * L_THRESHOLD);

pub global CHUNK_PUBLIC_INPUTS: u32 = SHARE_COMPUTATION_CHUNK_SIZE * L_THRESHOLD * (N_PARTIES + 1);

// Each batch proof bundles:
// - one base proof (`y` share-array layout for consistency),
// - CHUNKS_PER_BATCH chunk proofs,
// - batch_idx (which batch along the chunk sequence).

fn main(
    base_verification_key: UltraHonkVerificationKey,
    base_proof: UltraHonkZKProof,
    base_public_inputs: [Field; BASE_PUBLIC_INPUTS],
    base_key_hash: pub Field,
    chunk_verification_key: UltraHonkVerificationKey,
    chunk_proofs: [UltraHonkZKProof; CHUNKS_PER_BATCH],
    chunk_public_inputs: [[Field; CHUNK_PUBLIC_INPUTS]; CHUNKS_PER_BATCH],
    chunk_key_hash: pub Field,
    batch_idx: pub u32, // which batch this is
) -> pub Field {
    // Verify base proof
    verify_honk_proof(
        base_verification_key,
        base_proof,
        base_public_inputs,
        base_key_hash,
    );

    // Verify each chunk in this batch and enforce y consistency
    for i in 0..CHUNKS_PER_BATCH {
        verify_honk_proof(
            chunk_verification_key,
            chunk_proofs[i],
            chunk_public_inputs[i],
            chunk_key_hash,
        );

        let chunk_idx = batch_idx * CHUNKS_PER_BATCH + i;
        let base_y_start = 1 + chunk_idx * CHUNK_PUBLIC_INPUTS;
        for j in 0..CHUNK_PUBLIC_INPUTS {
            assert(
                base_public_inputs[base_y_start + j] == chunk_public_inputs[i][j],
                "y consistency check failed",
            );
        }
    }

    // Aggregate public inputs
    let mut aggregated_public_inputs = Vec::new();
    for i in 0..BASE_PUBLIC_INPUTS {
        aggregated_public_inputs.push(base_public_inputs[i]);
    }
    for i in 0..CHUNKS_PER_BATCH {
        for j in 0..CHUNK_PUBLIC_INPUTS {
            aggregated_public_inputs.push(chunk_public_inputs[i][j]);
        }
    }

    compute_recursive_aggregation_commitment(aggregated_public_inputs)
}
