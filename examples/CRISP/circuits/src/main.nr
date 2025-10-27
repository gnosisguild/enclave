// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use greco::{Greco, Params};
use polynomial::Polynomial;

mod ciphertext_addition;
use ciphertext_addition::check_ciphertext_addition_constraints;
use ciphertext_addition::check_range_bounds;
use ciphertext_addition::CiphertextAdditionParams;
mod ecdsa;
use ecdsa::{address_to_field, derive_address, verify_signature};
mod merkle_tree;
use merkle_tree::get_merkle_root;
mod utils;
use utils::check_coefficient_values;

fn main(
    ct_add_params: CiphertextAdditionParams<2048, 1>,
    params: pub Params<2048, 1>,
    pk0is: pub [Polynomial<2048>; 1],
    pk1is: pub [Polynomial<2048>; 1],
    ct0is: pub [Polynomial<2048>; 1],
    ct1is: pub [Polynomial<2048>; 1],
    u: Polynomial<2048>,
    e0: Polynomial<2048>,
    e1: Polynomial<2048>,
    k1: Polynomial<2048>,
    r1is: [Polynomial<4095>; 1],
    r2is: [Polynomial<2047>; 1],
    p1is: [Polynomial<4095>; 1],
    p2is: [Polynomial<2047>; 1],
    public_key_x: [u8; 32],
    public_key_y: [u8; 32],
    signature: [u8; 64],
    hashed_message: [u8; 32],
    balance: Field,
    merkle_root: pub Field,
    merkle_proof_length: u32,
    merkle_proof_indices: [u1; 20],
    merkle_proof_siblings: [Field; 20],
) {
    // First verify the signature.
    let is_signature_valid =
        verify_signature(hashed_message, public_key_x, public_key_y, signature);

    // Then derive the address.
    let address = address_to_field(derive_address(public_key_x, public_key_y));

    // Then get Merkle root.
    let merkle_root_calculated = get_merkle_root(
        address,
        balance,
        merkle_proof_length,
        merkle_proof_indices,
        merkle_proof_siblings,
    );

    // compare the merkle root with the param to confirm whether it's the voter
    // or a masker
    let mut is_voter = false;
    if ((merkle_root_calculated == merkle_root) & (is_signature_valid == true)) {
        is_voter = true;
    }

    // then verify that the vote was encrypted correctly
    let circuit: Greco<2048, 1, 54, 54, 5, 5, 20, 20, 54, 16, 54> = Greco::new(
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

    check_coefficient_values(k1, params.crypto_params().q_mod_t);

    let gammas = circuit.verify_correct_ciphertext_encryption();

    check_range_bounds::<2048, 1, 1>(ct_add_params);

    check_ciphertext_addition_constraints::<2048, 1>(
        ct_add_params,
        ct0is,
        ct1is,
        params.crypto_params().qis,
        gammas,
    );
}
