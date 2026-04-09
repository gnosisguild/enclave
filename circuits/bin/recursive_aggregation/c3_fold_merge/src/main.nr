// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Non-ZK merge step for the C3 binary fold tree: verifies two `c3_fold` / `c3_fold_merge` proofs
//! and combines their N-slot pk/msg arrays with disjointness + field addition.

use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};
use lib::configs::default::N_PARTIES;

fn main(
    vk1: UltraHonkVerificationKey,
    proof1: UltraHonkProof,
    pk_commits1: [Field; N_PARTIES],
    msg_commits1: [Field; N_PARTIES],
    key_hash1: pub Field,
    vk2: UltraHonkVerificationKey,
    proof2: UltraHonkProof,
    pk_commits2: [Field; N_PARTIES],
    msg_commits2: [Field; N_PARTIES],
    key_hash2: pub Field,
) -> pub ([Field; N_PARTIES], [Field; N_PARTIES]) {
    verify_honk_proof_non_zk(
        vk1,
        proof1,
        flatten_pair(pk_commits1, msg_commits1),
        key_hash1,
    );
    verify_honk_proof_non_zk(
        vk2,
        proof2,
        flatten_pair(pk_commits2, msg_commits2),
        key_hash2,
    );

    let mut pk_out: [Field; N_PARTIES] = [0; N_PARTIES];
    let mut msg_out: [Field; N_PARTIES] = [0; N_PARTIES];

    for i in 0..N_PARTIES {
        let pk_a = pk_commits1[i];
        let pk_b = pk_commits2[i];
        let msg_a = msg_commits1[i];
        let msg_b = msg_commits2[i];
        assert((pk_a == 0) | (pk_b == 0));
        assert((msg_a == 0) | (msg_b == 0));
        pk_out[i] = pk_a + pk_b;
        msg_out[i] = msg_a + msg_b;
    }

    (pk_out, msg_out)
}

fn flatten_pair(
    pk_commits: [Field; N_PARTIES],
    msg_commits: [Field; N_PARTIES],
) -> [Field; 2 * N_PARTIES] {
    let mut out: [Field; 2 * N_PARTIES] = [0; 2 * N_PARTIES];
    for i in 0..N_PARTIES {
        out[i] = pk_commits[i];
        out[N_PARTIES + i] = msg_commits[i];
    }
    out
}
