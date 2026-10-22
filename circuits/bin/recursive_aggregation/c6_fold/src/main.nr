// SPDX-License-Identifier: LGPL-3.0-only

use bb_proof_verification::{
    UltraHonkProof, UltraHonkVerificationKey, UltraHonkZKProof, verify_honk_proof,
    verify_honk_proof_non_zk,
};
use lib::configs::default::T;
use lib::configs::recursive::{C6_FOLD_PUBLIC_LEN, C6_INNER_PUBLIC_LEN, C6_KEY_SLOTS};
use lib::core::threshold::decryption_fold::fold_statement;
use lib::math::recursive_vk::predecessor_key_hash;

fn main(
    inner_vk: UltraHonkVerificationKey,
    inner_proof: UltraHonkZKProof,
    c6_public_inputs: [Field; C6_INNER_PUBLIC_LEN],
    acc_vk: UltraHonkVerificationKey,
    acc_proof: UltraHonkProof,
    acc_public_inputs: [Field; C6_FOLD_PUBLIC_LEN],
    inner_key_hash: pub Field,
    fold_key_hash: pub Field,
    kernel_key_hash: pub Field,
    is_first_step: pub bool,
    slot_index: pub u32,
    ) -> pub (Field, Field, [Field; T + 1], [Field; T + 1], [Field; T + 1], [Field; T + 1], [Field; C6_KEY_SLOTS], [Field; C6_KEY_SLOTS], [Field; T + 1]) {
    verify_honk_proof(inner_vk, inner_proof, c6_public_inputs, inner_key_hash);
    let key = predecessor_key_hash(
        inner_key_hash,
        fold_key_hash,
        kernel_key_hash,
        is_first_step,
        [acc_public_inputs[0], acc_public_inputs[1], acc_public_inputs[2]],
    );
    verify_honk_proof_non_zk(acc_vk, acc_proof, acc_public_inputs, key);
    fold_statement(
        c6_public_inputs,
        acc_public_inputs,
        is_first_step,
        slot_index,
    )
}
