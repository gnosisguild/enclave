// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use greco::{Greco, Params};
use polynomial::Polynomial;

mod ciphertext_addition;
use ciphertext_addition::CiphertextAddition;
mod ecdsa;
use ecdsa::{address_to_field, derive_address, verify_signature};
mod merkle_tree;
use merkle_tree::get_merkle_root;
mod utils;
use utils::{check_coefficient_values_with_balance, check_coefficient_zero};

fn main(
    // Ciphertext Addition Section.
    prev_ct0is: [Polynomial<512>; 2], // todo: make this pub
    prev_ct1is: [Polynomial<512>; 2], // todo: make this pub
    sum_ct0is: [Polynomial<512>; 2],
    sum_ct1is: [Polynomial<512>; 2],
    sum_r0is: [Polynomial<512>; 2],
    sum_r1is: [Polynomial<512>; 2],
    // Greco Section.
    params: Params<512, 2>, // todo: make this pub
    pk0is: [Polynomial<512>; 2],
    pk1is: [Polynomial<512>; 2],
    ct0is: [Polynomial<512>; 2],
    ct1is: [Polynomial<512>; 2],
    u: Polynomial<512>,
    e0: Polynomial<512>,
    e0is: [Polynomial<512>; 2],
    e0_quotients: [Polynomial<512>; 2],
    e1: Polynomial<512>,
    k1: Polynomial<512>,
    r1is: [Polynomial<1023>; 2],
    r2is: [Polynomial<511>; 2],
    p1is: [Polynomial<1023>; 2],
    p2is: [Polynomial<511>; 2],
    // ECDSA Section.
    public_key_x: [u8; 32],
    public_key_y: [u8; 32],
    signature: [u8; 64],
    hashed_message: [u8; 32],
    // Merkle Tree Section.
    merkle_root: Field, // todo: make this pub
    merkle_proof_length: u32,
    merkle_proof_indices: [u1; 20],
    merkle_proof_siblings: [Field; 20],
    // Slot Address Section.
    slot_address: pub Field,
    // Balance Section.
    balance: Field,
    // Whether this is the first vote for this slot.
    is_first_vote: pub bool,
) {
    // Verify the ECDSA signature.
    let is_signature_valid =
        verify_signature(hashed_message, public_key_x, public_key_y, signature);

    // Derive the Ethereum address.
    let address = address_to_field(derive_address(public_key_x, public_key_y));

    // Calculate the Merkle root.
    let merkle_root_calculated = get_merkle_root(
        address,
        balance,
        merkle_proof_length,
        merkle_proof_indices,
        merkle_proof_siblings,
    );

    let greco: Greco<512, 2, 36, 36, 2, 6, 6, 4, 10, 36, 10, 36> = Greco::new(
        params,
        pk0is,
        pk1is,
        ct0is,
        ct1is,
        u,
        e0,
        e0is,
        e0_quotients,
        e1,
        k1,
        r1is,
        r2is,
        p1is,
        p2is,
    );

    let ct_add: CiphertextAddition<512, 2, 36, 36, 36> = CiphertextAddition::new(
        params.crypto_params(),
        ct0is,
        ct1is,
        prev_ct0is,
        prev_ct1is,
        sum_ct0is,
        sum_ct1is,
        sum_r0is,
        sum_r1is,
    );

    // Verify the correct ciphertext encryption.
    let is_greco_valid = greco.verify();

    // Verify the correct ciphertext addition.
    let is_ct_add_valid = ct_add.verify();

    // Ensure that the ciphertext is valid
    assert(is_greco_valid);

    // If the voter is eligible to vote, output the ciphertext.
    // Otherwise, output the sum of the ciphertexts.
    if (is_signature_valid == true)
        & (merkle_root_calculated == merkle_root)
        & (slot_address == address) {
        // Verify the correct coefficient values and that the vote is <= balance
        check_coefficient_values_with_balance(k1, params.crypto_params().q_mod_t, balance);

        // (ct0is, ct1is)
    } else {
        // check if vote == 0.
        let is_vote_zero = check_coefficient_zero(k1);
        assert(is_vote_zero == true);

        // need to check if slot is empty (no previous ciphertext).
        // If so, (ct0is, ct1is) should be returned.

        if is_first_vote {
            // (ct0is, ct1is)
        } else {
            // check if the sum is valid
            assert(is_ct_add_valid);
            // (sum_ct0is, sum_ct1is)
        }
    }
}
