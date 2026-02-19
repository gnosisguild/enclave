// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};

// Number of proofs.
pub global N_PROOFS: u32 = 2;
/// Number of public inputs/outputs per proof.
pub global N_PUBLIC_INPUTS: u32 = 3;

fn main(
    verification_keys: [UltraHonkVerificationKey; N_PROOFS],
    proofs: [UltraHonkProof; N_PROOFS],
    public_inputs: pub [[Field; N_PUBLIC_INPUTS]; N_PROOFS],
    key_hashes: pub [Field; N_PROOFS],
) -> pub (Field, Field) {
    for i in 0..N_PROOFS {
        verify_honk_proof_non_zk(
            verification_keys[i],
            proofs[i],
            public_inputs[i],
            key_hashes[i],
        );
    }

    // compute commitment for (public_inputs[0][0], public_inputs[1][0]) pk
    let pk_commitment = 1;
    // compute commitment for (public_inputs[0][1], public_inputs[1][1]) ct
    let ct_commitment = 2;

    (pk_commitment, ct_commitment)
}
