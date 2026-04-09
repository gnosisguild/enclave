// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Base fold for C3 inner proofs: verifies one or two `ShareEncryption` ZK proofs and writes
//! `expected_pk_commitment` / `expected_message_commitment` into the given party slots.
//!
//! When `skip_second_proof` is set (odd leaf count), only `proof1` is verified; the second slot
//! is left zero and `proof2` inputs are ignored.

use bb_proof_verification::{UltraHonkVerificationKey, UltraHonkZKProof, verify_honk_proof};
use lib::configs::default::N_PARTIES;

fn main(
    vk: UltraHonkVerificationKey,
    proof1: UltraHonkZKProof,
    c3_public_inputs1: [Field; 3],
    key_hash: pub Field,
    slot_index1: pub u32,
    proof2: UltraHonkZKProof,
    c3_public_inputs2: [Field; 3],
    slot_index2: pub u32,
    skip_second_proof: pub bool,
) -> pub ([Field; N_PARTIES], [Field; N_PARTIES]) {
    assert(slot_index1 < N_PARTIES);
    assert(!skip_second_proof | (slot_index2 < N_PARTIES));
    assert(skip_second_proof | ((slot_index2 < N_PARTIES) & (slot_index1 != slot_index2)));

    verify_honk_proof(vk, proof1, c3_public_inputs1, key_hash);
    if !skip_second_proof {
        verify_honk_proof(vk, proof2, c3_public_inputs2, key_hash);
    }

    let mut pk_commits: [Field; N_PARTIES] = [0; N_PARTIES];
    let mut msg_commits: [Field; N_PARTIES] = [0; N_PARTIES];

    for i in 0..N_PARTIES {
        let idx = i as u32;
        let (pk_i, msg_i) = if skip_second_proof {
            if idx == slot_index1 {
                (c3_public_inputs1[0], c3_public_inputs1[1])
            } else {
                (0, 0)
            }
        } else if idx == slot_index1 {
            (c3_public_inputs1[0], c3_public_inputs1[1])
        } else if idx == slot_index2 {
            (c3_public_inputs2[0], c3_public_inputs2[1])
        } else {
            (0, 0)
        };
        pk_commits[i] = pk_i;
        msg_commits[i] = msg_i;
    }

    (pk_commits, msg_commits)
}
