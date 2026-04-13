// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sequential C3 fold: one inner ZK `ShareEncryption` proof per step, optionally chained with the
//! previous `c3_fold` non-ZK proof. First step (`is_first_step`) verifies only the inner proof.

use bb_proof_verification::{
    UltraHonkProof, UltraHonkVerificationKey, UltraHonkZKProof, verify_honk_proof,
    verify_honk_proof_non_zk,
};
use lib::configs::default::N_PARTIES;

/// Total public fields for a `c3_fold` proof: 4 pub parameters + 3*N party slots (pk, msg, ct).
pub global C3_FOLD_PUBLIC_LEN: u32 = 4 + (3 * N_PARTIES);

fn main(
    inner_vk: UltraHonkVerificationKey,
    inner_proof: UltraHonkZKProof,
    c3_public_inputs: [Field; 3],
    acc_vk: UltraHonkVerificationKey,
    acc_proof: UltraHonkProof,
    acc_public_inputs: [Field; C3_FOLD_PUBLIC_LEN],
    inner_key_hash: pub Field,
    acc_key_hash: pub Field,
    is_first_step: pub bool,
    slot_index: pub u32,
) -> pub ([Field; N_PARTIES], [Field; N_PARTIES], [Field; N_PARTIES]) {
    assert(slot_index < N_PARTIES);

    verify_honk_proof(inner_vk, inner_proof, c3_public_inputs, inner_key_hash);

    if !is_first_step {
        verify_honk_proof_non_zk(acc_vk, acc_proof, acc_public_inputs, acc_key_hash);
    }

    let pk_new = c3_public_inputs[0];
    let msg_new = c3_public_inputs[1];
    let ct_new = c3_public_inputs[2];

    let mut out_pk: [Field; N_PARTIES] = [0; N_PARTIES];
    let mut out_msg: [Field; N_PARTIES] = [0; N_PARTIES];
    let mut out_ct: [Field; N_PARTIES] = [0; N_PARTIES];

    for i in 0..N_PARTIES {
        let idx = i as u32;
        if is_first_step {
            if idx == slot_index {
                out_pk[i] = pk_new;
                out_msg[i] = msg_new;
                out_ct[i] = ct_new;
            } else {
                out_pk[i] = 0;
                out_msg[i] = 0;
                out_ct[i] = 0;
            }
        } else {
            let prior_pk = acc_public_inputs[4 + i];
            let prior_msg = acc_public_inputs[4 + N_PARTIES + i];
            let prior_ct = acc_public_inputs[4 + N_PARTIES + N_PARTIES + i];

            if idx == slot_index {
                assert(prior_pk == 0);
                assert(prior_msg == 0);
                assert(prior_ct == 0);
                out_pk[i] = pk_new;
                out_msg[i] = msg_new;
                out_ct[i] = ct_new;
            } else {
                out_pk[i] = prior_pk;
                out_msg[i] = prior_msg;
                out_ct[i] = prior_ct;
            }
        }
    }

    (out_pk, out_msg, out_ct)
}
