// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Level 2: final_wrapper
use bb_proof_verification::{UltraHonkVerificationKey, UltraHonkZKProof, verify_honk_proof};
use lib::configs::default::dkg::SHARE_COMPUTATION_N_BATCHES as N_BATCHES;

// Public inputs of each batch wrapper proof (as exposed by `share_computation_chunk_batch`).
// Currently: [base_key_hash, chunk_key_hash, batch_idx, aggregated_commitment].
pub global BATCH_WRAPPER_PUBLIC_INPUTS: u32 = 4;

fn main(
    batch_verification_key: UltraHonkVerificationKey,
    batch_proofs: [UltraHonkZKProof; N_BATCHES],
    batch_public_inputs: [[Field; BATCH_WRAPPER_PUBLIC_INPUTS]; N_BATCHES],
    batch_key_hash: pub Field,
) -> pub [Field; BATCH_WRAPPER_PUBLIC_INPUTS] {
    // 1. Verify all batch proofs against their public inputs and key hash.
    for i in 0..N_BATCHES {
        verify_honk_proof(
            batch_verification_key,
            batch_proofs[i],
            batch_public_inputs[i],
            batch_key_hash,
        );
    }

    // 2. Expose the batch wrapper public inputs (without recomputing a commitment)
    //    as the C2 public interface. We take them from the first batch, since each
    //    batch has the same shape.
    let mut c2_public_inputs: [Field; BATCH_WRAPPER_PUBLIC_INPUTS] =
        [0 as Field; BATCH_WRAPPER_PUBLIC_INPUTS];
    for i in 0..BATCH_WRAPPER_PUBLIC_INPUTS {
        c2_public_inputs[i] = batch_public_inputs[0][i];
    }

    c2_public_inputs
}
