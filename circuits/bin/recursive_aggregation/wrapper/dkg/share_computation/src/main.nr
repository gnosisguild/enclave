// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use bb_proof_verification::{UltraHonkVerificationKey, UltraHonkZKProof, verify_honk_proof};
use lib::math::commitments::compute_recursive_aggregation_commitment;

// Each SK/ESM final C2 proof is wrapped individually after the two-level pipeline.
pub global N_PROOFS: u32 = 1;
// The final share_computation circuit exposes 5 public outputs:
//   batch_key_hash (pub param) + [Field; 4] return (base_key_hash, chunk_key_hash, batch_idx, commitment).
pub global N_PUBLIC_INPUTS: u32 = 5;

fn main(
    verification_key: UltraHonkVerificationKey,
    proofs: [UltraHonkZKProof; N_PROOFS],
    public_inputs: [[Field; N_PUBLIC_INPUTS]; N_PROOFS],
    key_hash: pub Field,
) -> pub Field {
    for i in 0..N_PROOFS {
        verify_honk_proof(verification_key, proofs[i], public_inputs[i], key_hash);
    }

    let mut aggregated_public_inputs = Vec::new();

    for i in 0..N_PROOFS {
        for j in 0..N_PUBLIC_INPUTS {
            aggregated_public_inputs.push(public_inputs[i][j]);
        }
    }

    compute_recursive_aggregation_commitment(aggregated_public_inputs)
}
