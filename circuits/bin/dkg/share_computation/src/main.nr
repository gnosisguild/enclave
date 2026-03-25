// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Level 2: final_wrapper
use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};
use lib::configs::default::dkg::{
    SHARE_COMPUTATION_EXPECTED_VK_HASH_ESM, SHARE_COMPUTATION_EXPECTED_VK_HASH_SK,
    SHARE_COMPUTATION_N_BATCHES as N_BATCHES,
};
use lib::math::commitments::{compute_recursive_aggregation_commitment, compute_vk_hash};

// Public inputs of each batch wrapper proof (as exposed by `share_computation_chunk_batch`).
// Layout: [base_key_hash, chunk_key_hash, batch_idx, aggregated_commitment].
pub global BATCH_WRAPPER_PUBLIC_INPUTS: u32 = 4;

fn main(
    batch_verification_key: UltraHonkVerificationKey,
    batch_proofs: [UltraHonkProof; N_BATCHES],
    batch_public_inputs: [[Field; BATCH_WRAPPER_PUBLIC_INPUTS]; N_BATCHES],
    batch_key_hash: pub Field,
) -> pub (Field, Field) {
    // 1. Verify all batch proofs (non-zk).
    for i in 0..N_BATCHES {
        verify_honk_proof_non_zk(
            batch_verification_key,
            batch_proofs[i],
            batch_public_inputs[i],
            batch_key_hash,
        );
    }

    // 2. Assert shared fields are identical across all batches.
    for i in 1..N_BATCHES {
        assert(
            batch_public_inputs[i][0] == batch_public_inputs[0][0],
            "base_key_hash mismatch across batches",
        );
        assert(
            batch_public_inputs[i][1] == batch_public_inputs[0][1],
            "chunk_key_hash mismatch across batches",
        );
    }

    // 3. Assert batch_idx values are ordered 0..N_BATCHES.
    for i in 0..N_BATCHES {
        assert(batch_public_inputs[i][2] == i as Field, "batch_idx out of order");
    }

    // 4. Fold all per-batch aggregated_commitment values into a single commitment.
    let mut commitments = Vec::new();
    for i in 0..N_BATCHES {
        commitments.push(batch_public_inputs[i][3]);
    }
    let final_commitment = compute_recursive_aggregation_commitment(commitments);

    // 5. Hash the full VK chain: inner VK hashes (base, chunk) from batch public
    //    inputs + the batch VK hash that verified this level. This combined fingerprint
    //    lets the verifier check the entire proof genealogy.
    let mut vk_hashes: Vec<Field> = Vec::new();
    vk_hashes.push(batch_public_inputs[0][0]); // base_key_hash (same across all batches)
    vk_hashes.push(batch_public_inputs[0][1]); // chunk_key_hash (same across all batches)
    vk_hashes.push(batch_key_hash); // VK hash of the batch circuit that produced these proofs
    let key_hash = compute_vk_hash(vk_hashes);
    assert(
        (key_hash == SHARE_COMPUTATION_EXPECTED_VK_HASH_SK)
            | (key_hash == SHARE_COMPUTATION_EXPECTED_VK_HASH_ESM),
    );

    (key_hash, final_commitment)
}
