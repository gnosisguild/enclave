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
use utils::{check_coefficient_values, check_coefficient_zero};

fn main(
    // Ciphertext Addition Section.
    prev_ct0is: pub [Polynomial<2048>; 1],
    prev_ct1is: pub [Polynomial<2048>; 1],
    sum_ct0is: [Polynomial<2048>; 1],
    sum_ct1is: [Polynomial<2048>; 1],
    sum_r0is: [Polynomial<2048>; 1],
    sum_r1is: [Polynomial<2048>; 1],
    // Greco Section.
    params: pub Params<2048, 1>,
    pk0is: pub [Polynomial<2048>; 1],
    pk1is: pub [Polynomial<2048>; 1],
    ct0is: [Polynomial<2048>; 1],
    ct1is: [Polynomial<2048>; 1],
    u: Polynomial<2048>,
    e0: Polynomial<2048>,
    e1: Polynomial<2048>,
    k1: Polynomial<2048>,
    r1is: [Polynomial<4095>; 1],
    r2is: [Polynomial<2047>; 1],
    p1is: [Polynomial<4095>; 1],
    p2is: [Polynomial<2047>; 1],
    // ECDSA Section.
    public_key_x: [u8; 32],
    public_key_y: [u8; 32],
    signature: [u8; 64],
    hashed_message: [u8; 32],
    // Merkle Tree Section.
    merkle_root: pub Field,
    merkle_proof_length: u32,
    merkle_proof_indices: [u1; 20],
    merkle_proof_siblings: [Field; 20],
    // Slot Address Section.
    slot_address: pub Field,
    // Balance Section.
    balance: Field,
) -> pub ([Polynomial<2048>; 1], [Polynomial<2048>; 1]) {
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

    println(merkle_root_calculated);

    let greco: Greco<2048, 1, 54, 54, 5, 5, 20, 20, 54, 16, 54> = Greco::new(
        params,
        pk0is,
        pk1is,
        ct0is,
        ct1is,
        u,
        e0,
        e1,
        k1,
        r1is,
        r2is,
        p1is,
        p2is,
    );

    let ct_add: CiphertextAddition<2048, 1, 54, 54, 54> = CiphertextAddition::new(
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

    // assert(is_greco_valid == true);

    // Verify the correct ciphertext addition.
    let is_ct_add_valid = ct_add.verify();

    // Ensure that the ciphertext is valid
    assert(is_greco_valid);

    // If the voter is eligible to vote, output the ciphertext.
    // Otherwise, output the sum of the ciphertexts.
    if (is_signature_valid == true)
        & (merkle_root_calculated == merkle_root)
        & (slot_address == address) {
        // @todo: need to check if vote <= balance.

        // Verify the correct coefficient values.
        check_coefficient_values(k1, params.crypto_params().q_mod_t);

        (ct0is, ct1is)
    } else {
        // check if vote == 0.
        check_coefficient_zero(k1);

        // @todo: need to check if slot is empty (no previous ciphertext).
        // If so, (ct0is, ct1is) should be returned.

        // as well as the sum
        assert(is_ct_add_valid);

        (sum_ct0is, sum_ct1is)
    }
}
