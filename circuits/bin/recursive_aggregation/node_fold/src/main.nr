// SPDX-License-Identifier: LGPL-3.0-only

//! Bind one dealer's C0/C1/C2, complete C3 chain, and recipient C4 statement.

use bb_proof_verification::{
    UltraHonkProof, UltraHonkVerificationKey, UltraHonkZKProof, verify_honk_proof,
    verify_honk_proof_non_zk,
};
use lib::configs::default::dkg::L_THRESHOLD;
use lib::configs::default::{H, N_PARTIES};
use lib::configs::recursive::{
    C2_PUBLIC_LEN, C3_FOLD_PREFIX_LEN, C3_FOLD_PUBLIC_LEN, C3_SLOTS, C4_PUBLIC_LEN,
};
use lib::math::commitments::compute_vk_hash;

fn assert_c3_recipient_keys(party_id: Field, c3: [Field; C3_FOLD_PUBLIC_LEN]) {
    for recipient in 0..N_PARTIES {
        for row in 0..L_THRESHOLD {
            if party_id != recipient as Field {
                assert(
                    c3[C3_FOLD_PREFIX_LEN + recipient * L_THRESHOLD + row]
                        == c3[C3_FOLD_PREFIX_LEN + recipient * L_THRESHOLD],
                    "Recipient key differs between limbs",
                );
            }
        }
    }
}

fn main(
    c0_vk: UltraHonkVerificationKey,
    c0_proof: UltraHonkZKProof,
    c0_public: [Field; 1],
    c1_vk: UltraHonkVerificationKey,
    c1_proof: UltraHonkZKProof,
    c1_public: [Field; 2],
    c2_vk: UltraHonkVerificationKey,
    c2_proof: UltraHonkZKProof,
    c2_public: [Field; C2_PUBLIC_LEN],
    c3_vk: UltraHonkVerificationKey,
    c3_proof: UltraHonkProof,
    c3_public: [Field; C3_FOLD_PUBLIC_LEN],
    c4_vk: UltraHonkVerificationKey,
    c4_proof: UltraHonkZKProof,
    c4_public: [Field; C4_PUBLIC_LEN],
    party_id: pub Field,
    c0_key_hash: pub Field,
    c1_key_hash: pub Field,
    c2_key_hash: pub Field,
    c3_key_hash: pub Field,
    c4_key_hash: pub Field,
    ) -> pub (Field, Field, Field, [Field; N_PARTIES], [[Field; L_THRESHOLD]; N_PARTIES], [[Field; L_THRESHOLD]; H], Field, [Field; N_PARTIES], [Field; H]) {
    party_id.assert_max_bit_size::<32>();
    assert((party_id as u32) < N_PARTIES);
    verify_honk_proof(c0_vk, c0_proof, c0_public, c0_key_hash);
    verify_honk_proof(c1_vk, c1_proof, c1_public, c1_key_hash);
    verify_honk_proof(c2_vk, c2_proof, c2_public, c2_key_hash);
    verify_honk_proof_non_zk(c3_vk, c3_proof, c3_public, c3_key_hash);
    verify_honk_proof(c4_vk, c4_proof, c4_public, c4_key_hash);
    assert(c3_public[1] == c3_key_hash);
    let c3_tree = compute_vk_hash([c3_key_hash, c3_public[2], c3_public[0]].as_vector());
    let tree = compute_vk_hash([
        c0_key_hash,
        c1_key_hash,
        c2_key_hash,
        c3_key_hash,
        c3_tree,
        c4_key_hash,
    ]
        .as_vector());
    assert(c1_public[0] == c2_public[0], "C1/C2 secret mismatch");
    assert(c4_public[H * L_THRESHOLD] == party_id, "C4 recipient mismatch");
    assert_c3_recipient_keys(party_id, c3_public);
    let mut recipient_keys = [0; N_PARTIES];
    let mut messages = [[0; L_THRESHOLD]; N_PARTIES];
    let mut outgoing = [0; N_PARTIES];
    for recipient in 0..N_PARTIES {
        outgoing[recipient] = c2_public[1 + recipient];
        recipient_keys[recipient] = if party_id == recipient as Field {
            c0_public[0]
        } else {
            c3_public[C3_FOLD_PREFIX_LEN + recipient * L_THRESHOLD]
        };
        for row in 0..L_THRESHOLD {
            let slot = recipient * L_THRESHOLD + row;
            messages[recipient][row] = c2_public[1 + N_PARTIES + slot];
            if party_id != recipient as Field {
                assert(
                    messages[recipient][row] == c3_public[C3_FOLD_PREFIX_LEN + C3_SLOTS + slot],
                    "C2/C3 message mismatch",
                );
                assert(
                    outgoing[recipient] == c3_public[C3_FOLD_PREFIX_LEN + 3 * C3_SLOTS + slot],
                    "C2/C3 PRF key mismatch",
                );
            }
        }
    }
    let mut received = [[0; L_THRESHOLD]; H];
    let mut incoming = [0; H];
    for dealer in 0..H {
        for row in 0..L_THRESHOLD {
            received[dealer][row] = c4_public[dealer * L_THRESHOLD + row];
        }
        incoming[dealer] = c4_public[H * L_THRESHOLD + 1 + dealer];
    }
    let aggregate = c4_public[C4_PUBLIC_LEN - 1];
    (
        tree, c0_public[0], c1_public[1], recipient_keys, messages, received, aggregate, outgoing,
        incoming,
    )
}

#[test]
fn consistent_c3_recipient_keys_are_accepted() {
    let mut c3 = [0; C3_FOLD_PUBLIC_LEN];
    for j in 0..N_PARTIES {
        for l in 0..L_THRESHOLD {
            c3[C3_FOLD_PREFIX_LEN + j * L_THRESHOLD + l] = j as Field + 1000;
        }
    }
    assert_c3_recipient_keys(0, c3);
}

#[test(should_fail)]
fn divergent_nonzero_limb_c3_key_is_rejected() {
    let mut c3 = [0; C3_FOLD_PUBLIC_LEN];
    c3[C3_FOLD_PREFIX_LEN + L_THRESHOLD + 1] = 1;
    assert_c3_recipient_keys(0, c3);
}

#[test]
fn self_recipient_limbs_may_differ() {
    let mut c3 = [0; C3_FOLD_PUBLIC_LEN];
    c3[C3_FOLD_PREFIX_LEN + 1] = 1;
    assert_c3_recipient_keys(0, c3);
}
