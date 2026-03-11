// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Base and chunk proofs are produced with noir-recursive (ZK) target, so they use
// UltraHonkZKProof (508 elements). Non-ZK type would expect 457 and cause LengthMismatch.
use bb_proof_verification::{UltraHonkVerificationKey, UltraHonkZKProof, verify_honk_proof};
use lib::configs::default::dkg::{
    L_THRESHOLD, N, SHARE_COMPUTATION_CHUNK_SIZE, SHARE_COMPUTATION_N_CHUNKS,
};
use lib::configs::default::N_PARTIES;
use lib::math::commitments::compute_recursive_aggregation_commitment;

// 1 base proof + N_CHUNKS chunk proofs
pub global N_PROOFS: u32 = 1 + SHARE_COMPUTATION_N_CHUNKS;

// Base proof public inputs:
// expected_secret_commitment (1) + y (N * L_THRESHOLD * (N_PARTIES+1)) + party_commitments (N_PARTIES * L_THRESHOLD)
pub global BASE_PUBLIC_INPUTS: u32 =
    1 + (N * L_THRESHOLD * (N_PARTIES + 1)) + (N_PARTIES * L_THRESHOLD);

// Chunk proof public inputs:
// y_chunk (CHUNK_SIZE * L_THRESHOLD * (N_PARTIES+1))
pub global CHUNK_PUBLIC_INPUTS: u32 = SHARE_COMPUTATION_CHUNK_SIZE * L_THRESHOLD * (N_PARTIES + 1);

fn main(
    base_verification_key: UltraHonkVerificationKey,
    base_proof: UltraHonkZKProof,
    base_public_inputs: [Field; BASE_PUBLIC_INPUTS],
    base_key_hash: pub Field,
    chunk_verification_key: UltraHonkVerificationKey,
    chunk_proofs: [UltraHonkZKProof; SHARE_COMPUTATION_N_CHUNKS],
    chunk_public_inputs: [[Field; CHUNK_PUBLIC_INPUTS]; SHARE_COMPUTATION_N_CHUNKS],
    chunk_key_hash: pub Field,
) -> pub Field {
    // Step 1: Verify base proof (ZK verifier; proof is 508 elements)
    verify_honk_proof(
        base_verification_key,
        base_proof,
        base_public_inputs,
        base_key_hash,
    );

    // Step 2: Verify each chunk proof and enforce y consistency with base
    // y starts at index 1 in base_public_inputs (after expected_secret_commitment)
    // y_chunk for chunk i starts at offset i * CHUNK_PUBLIC_INPUTS within base y
    for chunk_idx in 0..SHARE_COMPUTATION_N_CHUNKS {
        verify_honk_proof(
            chunk_verification_key,
            chunk_proofs[chunk_idx],
            chunk_public_inputs[chunk_idx],
            chunk_key_hash,
        );

        // Enforce y consistency: base_public_inputs y slice == chunk y_chunk
        let base_y_start = 1 + chunk_idx * CHUNK_PUBLIC_INPUTS;
        for i in 0..CHUNK_PUBLIC_INPUTS {
            assert(
                base_public_inputs[base_y_start + i] == chunk_public_inputs[chunk_idx][i],
                "y consistency check failed between base and chunk",
            );
        }
    }

    // Step 3: Aggregate all public inputs into a single commitment
    let mut aggregated_public_inputs = Vec::new();
    for i in 0..BASE_PUBLIC_INPUTS {
        aggregated_public_inputs.push(base_public_inputs[i]);
    }
    for chunk_idx in 0..SHARE_COMPUTATION_N_CHUNKS {
        for i in 0..CHUNK_PUBLIC_INPUTS {
            aggregated_public_inputs.push(chunk_public_inputs[chunk_idx][i]);
        }
    }

    compute_recursive_aggregation_commitment(aggregated_public_inputs)
}
