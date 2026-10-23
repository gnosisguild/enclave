// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sequential fold of `H` per-node `node_fold` proofs (non-ZK), one honest slot per step, chained with
//! the previous `nodes_fold` non-ZK proof, or a fixed genesis proof from `nodes_fold_kernel` on the
//! first step (see `crates/zk/prover` and `nodes_fold_kernel`).

use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};
use lib::configs::default::H;
use lib::configs::recursive::{
    NODE_FOLD_PUBLIC_LEN, NODES_FOLD_PARAM_PUBLIC_LEN, NODES_FOLD_PUBLIC_LEN,
};
use lib::math::recursive_vk::predecessor_key_hash;

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

    let acc_key_hash = predecessor_key_hash(
        inner_key_hash,
        fold_key_hash,
        kernel_key_hash,
        is_first_step,
        [acc_public_inputs[0], acc_public_inputs[1], acc_public_inputs[2]],
    );
    verify_honk_proof_non_zk(acc_vk, acc_proof, acc_public_inputs, acc_key_hash);

    let mut out: [[Field; NODE_FOLD_PUBLIC_LEN]; H] = [[0; NODE_FOLD_PUBLIC_LEN]; H];

    for i in 0..H {
        let idx = i as u32;
        if is_first_step {
            if idx == slot_index {
                out[i] = node_fold_public_inputs;
            }
        } else if idx == slot_index {
            for k in 0..NODE_FOLD_PUBLIC_LEN {
                assert(
                    acc_public_inputs[NODES_FOLD_PARAM_PUBLIC_LEN + i * NODE_FOLD_PUBLIC_LEN + k]
                        == 0,
                );
            }
            out[i] = node_fold_public_inputs;
        } else {
            for k in 0..NODE_FOLD_PUBLIC_LEN {
                out[i][k] =
                    acc_public_inputs[NODES_FOLD_PARAM_PUBLIC_LEN + i * NODE_FOLD_PUBLIC_LEN + k];
            }
        }
    }

    out
}
