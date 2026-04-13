// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Combines the final `c3_fold` proof for sk C3 chain with the final `c3_fold` for e_sm.

use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};
use lib::configs::default::N_PARTIES;
use lib::math::commitments::compute_vk_hash;

/// Must match `c3_fold`: `4 + 3*N_PARTIES` public fields per final proof.
pub global C3_FOLD_PUBLIC_LEN: u32 = 4 + (3 * N_PARTIES);

/// Total public fields for `c3ab_fold` (two key hashes + flattened return tuple).
pub global C3AB_FOLD_PUBLIC_LEN: u32 = 2 + 1 + (6 * N_PARTIES);

fn main(
    c3a_vk: UltraHonkVerificationKey,
    c3a_proof: UltraHonkProof,
    c3a_public: [Field; C3_FOLD_PUBLIC_LEN],
    c3b_vk: UltraHonkVerificationKey,
    c3b_proof: UltraHonkProof,
    c3b_public: [Field; C3_FOLD_PUBLIC_LEN],
    c3a_key_hash: pub Field,
    c3b_key_hash: pub Field,
    ) -> pub (Field, [Field; N_PARTIES], [Field; N_PARTIES], [Field; N_PARTIES], [Field; N_PARTIES], [Field; N_PARTIES], [Field; N_PARTIES]) {
    verify_honk_proof_non_zk(c3a_vk, c3a_proof, c3a_public, c3a_key_hash);
    verify_honk_proof_non_zk(c3b_vk, c3b_proof, c3b_public, c3b_key_hash);

    let mut vk_hashes = Vec::new();
    vk_hashes.push(c3a_key_hash);
    vk_hashes.push(c3b_key_hash);
    let key_hash = compute_vk_hash(vk_hashes);

    let mut pk_a: [Field; N_PARTIES] = [0; N_PARTIES];
    let mut msg_a: [Field; N_PARTIES] = [0; N_PARTIES];
    let mut ct_a: [Field; N_PARTIES] = [0; N_PARTIES];
    let mut pk_b: [Field; N_PARTIES] = [0; N_PARTIES];
    let mut msg_b: [Field; N_PARTIES] = [0; N_PARTIES];
    let mut ct_b: [Field; N_PARTIES] = [0; N_PARTIES];

    for j in 0..N_PARTIES {
        pk_a[j] = c3a_public[4 + j];
        msg_a[j] = c3a_public[4 + N_PARTIES + j];
        ct_a[j] = c3a_public[4 + N_PARTIES + N_PARTIES + j];
        pk_b[j] = c3b_public[4 + j];
        msg_b[j] = c3b_public[4 + N_PARTIES + j];
        ct_b[j] = c3b_public[4 + N_PARTIES + N_PARTIES + j];
    }

    (key_hash, pk_a, msg_a, ct_a, pk_b, msg_b, ct_b)
}
