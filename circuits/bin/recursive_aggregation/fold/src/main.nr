// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};
use lib::math::commitments::compute_recursive_aggregation_commitment;

fn main(
    proof1_verification_key: UltraHonkVerificationKey,
    proof1_proof: UltraHonkProof,
    proof1_commitment: Field,
    proof1_key_hash: Field,
    proof2_verification_key: UltraHonkVerificationKey,
    proof2_proof: UltraHonkProof,
    proof2_commitment: Field,
    proof2_key_hash: Field,
) -> pub Field {
    verify_honk_proof_non_zk(
        proof1_verification_key,
        proof1_proof,
        [proof1_commitment],
        proof1_key_hash,
    );
    verify_honk_proof_non_zk(
        proof2_verification_key,
        proof2_proof,
        [proof2_commitment],
        proof2_key_hash,
    );

    let mut commitments_vec = Vec::new();
    commitments_vec.push(proof1_commitment);
    commitments_vec.push(proof2_commitment);

    compute_recursive_aggregation_commitment(commitments_vec)
}
