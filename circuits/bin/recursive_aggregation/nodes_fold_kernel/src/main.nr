// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Same IO as [`nodes_fold`](../nodes_fold/src/main.nr) but **only** verifies the inner `node_fold`
//! proof. Used to mint a fixed genesis `UltraHonkProof` that bootstraps the real `nodes_fold`
//! circuit, which always runs `verify_honk_proof_non_zk` on the accumulator (see `nodes_fold`).

use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};
use lib::configs::default::H;
use lib::configs::recursive::{NODE_FOLD_PUBLIC_LEN, NODES_FOLD_PUBLIC_LEN};

fn main(
    inner_vk: UltraHonkVerificationKey,
    inner_proof: UltraHonkProof,
    node_fold_public_inputs: [Field; NODE_FOLD_PUBLIC_LEN],
    acc_vk: UltraHonkVerificationKey,
    acc_proof: UltraHonkProof,
    acc_public_inputs: [Field; NODES_FOLD_PUBLIC_LEN],
    inner_key_hash: pub Field,
    fold_key_hash: pub Field,
    kernel_key_hash: pub Field,
    is_first_step: pub bool,
    slot_index: pub u32,
) -> pub [[Field; NODE_FOLD_PUBLIC_LEN]; H] {
    assert(slot_index < H);

    verify_honk_proof_non_zk(
        inner_vk,
        inner_proof,
        node_fold_public_inputs,
        inner_key_hash,
    );

    // Signature parity with `nodes_fold`: see `c3_fold_kernel`.
    let _ = (acc_vk, acc_proof, fold_key_hash, kernel_key_hash, acc_public_inputs);

    assert(is_first_step);

    let mut out: [[Field; NODE_FOLD_PUBLIC_LEN]; H] = [[0; NODE_FOLD_PUBLIC_LEN]; H];

    for i in 0..H {
        let idx = i as u32;
        if idx == slot_index {
            out[i] = node_fold_public_inputs;
        }
    }

    out
}
