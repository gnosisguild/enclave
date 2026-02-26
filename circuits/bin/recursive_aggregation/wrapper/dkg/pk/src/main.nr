// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};
use lib::math::commitments::compute_recursive_aggregation_commitment;

// Number of proofs.
pub global N_PROOFS: u32 = 1;
/// Number of public inputs/outputs per proof.
pub global N_PUBLIC_INPUTS: u32 = 1;

fn main(
    verification_key: UltraHonkVerificationKey,
    proofs: [UltraHonkProof; N_PROOFS],
    public_inputs: pub [[Field; N_PUBLIC_INPUTS]; N_PROOFS],
    key_hash: pub Field,
) -> pub Field {
    for i in 0..N_PROOFS {
        verify_honk_proof_non_zk(verification_key, proofs[i], public_inputs[i], key_hash);
    }

    let mut aggregated_public_inputs = Vec::new();

    for i in 0..N_PROOFS {
        for j in 0..N_PUBLIC_INPUTS {
            aggregated_public_inputs.push(public_inputs[i][j]);
        }
    }

    compute_recursive_aggregation_commitment(aggregated_public_inputs)
}
