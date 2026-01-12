// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use greco::{Greco, Params};
use polynomial::Polynomial;

mod hash;
use hash::generate_commitment;
mod constants;
mod ciphertext_addition;
use ciphertext_addition::CiphertextAddition;
mod ecdsa;
use ecdsa::{address_to_field, derive_address, verify_signature};
mod merkle_tree;
use merkle_tree::get_merkle_root;
mod utils;
use utils::{check_coefficient_values_with_balance, check_coefficient_zero};

/// Main circuit function for CRISP vote verification.
///
/// This circuit implements the core verification logic for the CRISP (Coercion-Resistant Impartial
/// Selection Protocol) voting system. It verifies that votes are properly encrypted using BFV
/// (Brakerski-Fan-Vercauteren) fully homomorphic encryption and handles two distinct use cases:
/// 1. **Actual Voting**: An eligible voter casts a vote
/// 2. **Mask Vote**: Anyone submits a zero vote to mask slot activity
///
/// Votes are stored in an on-chain Merkle tree, where each vote ciphertext is stored at a specific
/// index. A mapping stores the slot address (Ethereum address) to the index in the Merkle tree.
///   - For actual votes: the slot address must match the address derived from the signature
///   - For mask votes: the slot address can be any address (no address match required)
/// In both cases, the address of the slot must be in the eligibility Merkle tree, a tree used
/// to represent all token holders eligible to vote.
///
/// Mask votes prevent receipt sharing attacks by creating ambiguity: anyone can submit a zero vote
/// to any slot, creating a different ciphertext that decrypts to the same value. This makes it
/// impossible for a voter to prove which ciphertext was their actual vote, preventing them from
/// providing a verifiable receipt to a coercer or briber.
///
/// The circuit verifies all inputs and returns the appropriate ciphertext.
/// Specifically, it returns a tuple of ciphertext components depending on the case:
/// - **Actual vote**: `(ct0is, ct1is)` - the new vote ciphertext
/// - **Mask vote (first vote)**: `(ct0is, ct1is)` - zero ciphertext
/// - **Mask vote (updating slot)**: `(sum_ct0is, sum_ct1is)` - sum of previous votes in slot
///
fn main(
    // Ciphertext Addition Section
    prev_ct0is: [Polynomial<512>; 2],
    prev_ct1is: [Polynomial<512>; 2],
    prev_ct_commitment: pub Field,
    sum_ct0is: [Polynomial<512>; 2],
    sum_ct1is: [Polynomial<512>; 2],
    sum_r0is: [Polynomial<512>; 2],
    sum_r1is: [Polynomial<512>; 2],
    // Greco Section.
    params: Params<512, 2>, // todo: make this pub
    pk_commitment: pub Field,
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
    merkle_root: pub Field, // todo: make this pub
    merkle_proof_length: u32,
    merkle_proof_indices: [u1; 20],
    merkle_proof_siblings: [Field; 20],
    // Other inputs.
    slot_address: pub Field,
    balance: Field,
    is_first_vote: pub bool,
) -> pub Field {
    // ============================================================================
    // STEP 1: Authentication - Derive voter's address from signature
    // ============================================================================
    // Verify that a constant message was signed by the holder of the private key
    // corresponding to the provided public key. Then the address is derived from
    // that public key and compared later to the slot address if the vote is an actual
    // vote.
    //
    // For ACTUAL VOTES: Signature must be valid (voter authenticates themselves)
    // For MASK VOTES: Signature may be invalid (anyone can submit mask votes)
    let is_signature_valid =
        verify_signature(hashed_message, public_key_x, public_key_y, signature);
    let address = address_to_field(derive_address(public_key_x, public_key_y));

    // ============================================================================
    // STEP 2: Eligibility - Merkle Tree Proof
    // ============================================================================
    // Verify that the (slot_address, balance) pair exists in the eligibility
    // Merkle tree. The proof demonstrates membership without revealing the entire tree.
    // The leaf is computed as poseidon_hash(address, balance). The root is compared to
    // the expected root, which is stored in the contract and passed as a public input
    // when verifying the proof.
    //
    // This check applies to BOTH cases:
    // - For actual votes: verifies the voter is eligible
    // - For mask votes: verifies the slot address is in the eligibility tree
    let merkle_root_calculated = get_merkle_root(
        slot_address,
        balance,
        merkle_proof_length,
        merkle_proof_indices,
        merkle_proof_siblings,
    );

    assert(merkle_root_calculated == merkle_root);

    // ============================================================================
    // STEP 3: BFV Encryption Verification (GRECO)
    // ============================================================================
    // Verify that the ciphertext (ct0is, ct1is) is a valid encryption of the
    // plaintext k1 under the public key (pk0is, pk1is). This enusres the ciphertext
    // is correctly formed.
    //
    // This check applies to BOTH cases:
    // - For actual votes: verifies the vote ciphertext is correctly formed
    // - For mask votes: verifies the zero vote ciphertext is correctly formed
    let greco: Greco<512, 2, 36, 36, 2, 6, 6, 4, 10, 36, 10, 36> = Greco::new(
        params,
        pk_commitment,
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

    assert(greco.verify());

    // ============================================================================
    // STEP 4: Ciphertext Addition Verification
    // ============================================================================
    // Verify that the sum ciphertext correctly represents the homomorphic addition
    // of a zero vote to the previous ciphertext without decrypting. This is only checked
    // for mask votes, and only when the slot is not empty.
    //
    // Mask votes add zero to the previous ciphertext, creating a different ciphertext
    // with the same plaintext as the previous ciphertext.
    //
    // Commitments are cryptographic hashes of the polynomial coefficients that bind the
    // prover to specific ciphertext values. Three commitments are generated:
    // - prev_ct_commitment: commitment to the previous ciphertext (prev_ct0is, prev_ct1is)
    // - ct_commitment: commitment to the new vote ciphertext (ct0is, ct1is)
    // - sum_ct_commitment: commitment to the sum ciphertext (sum_ct0is, sum_ct1is)
    //
    // The prev_ct_commitment is verified against the actual polynomials to ensure the
    // prover hasn't tampered with the previous ciphertext. This check is only performed
    // for mask votes when it's not the first vote (i.e., when there's a previous ciphertext
    // to verify). The commitments are then used in the Fiat-Shamir transform to generate
    // random challenges for the Schwartz-Zippel lemma verification, ensuring the addition
    // equations hold without revealing the full polynomial coefficients.
    let _prev_ct_commitment = generate_commitment::<512, 2, 36>(prev_ct0is, prev_ct1is);
    let ct_commitment = generate_commitment::<512, 2, 36>(ct0is, ct1is);
    let sum_ct_commitment = generate_commitment::<512, 2, 36>(sum_ct0is, sum_ct1is);

    let ct_add: CiphertextAddition<512, 2, 36, 36, 36> = CiphertextAddition::new(
        params.crypto_params(),
        ct0is,
        ct1is,
        ct_commitment,
        prev_ct0is,
        prev_ct1is,
        _prev_ct_commitment,
        sum_ct0is,
        sum_ct1is,
        sum_ct_commitment,
        sum_r0is,
        sum_r1is,
    );

    let is_ct_add_valid = ct_add.verify();

    // ============================================================================
    // STEP 5: Vote Type Detection and Return Logic
    // ============================================================================
    // The circuit branches into two cases based on signature validity and address match:
    //
    // CASE 1: ACTUAL VOTE
    //   Condition: Signature valid AND address matches slot
    //   - This is an eligible voter casting a vote
    //   - Verify vote amount <= balance
    //   - Return new vote ciphertext (ct0is, ct1is)
    //
    // CASE 2: MASK VOTE
    //   Condition: Signature invalid OR address mismatch
    //   - This is anyone submitting a zero vote to mask slot activity
    //   - Verify vote is actually zero (k1 must be zero)
    //   - If first vote in slot: return zero ciphertext (ct0is, ct1is)
    //   - If updating slot: verify addition and return sum of previous votes
    if (is_signature_valid == true) & (slot_address == address) {
        check_coefficient_values_with_balance(k1, params.crypto_params().q_mod_t, balance);

        ct_commitment
    } else {
        let is_vote_zero = check_coefficient_zero(k1);
        assert(is_vote_zero == true);

        if is_first_vote {
            ct_commitment
        } else {
            assert(prev_ct_commitment == _prev_ct_commitment);
            assert(is_ct_add_valid);

            sum_ct_commitment
        }
    }
}
