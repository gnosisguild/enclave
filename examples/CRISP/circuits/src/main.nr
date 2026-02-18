// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::default::threshold::{
    L, N, Q_MOD_T_CENTERED, USER_DATA_ENCRYPTION_BIT_CT, USER_DATA_ENCRYPTION_BIT_E0,
    USER_DATA_ENCRYPTION_BIT_E1, USER_DATA_ENCRYPTION_BIT_K, USER_DATA_ENCRYPTION_BIT_P1,
    USER_DATA_ENCRYPTION_BIT_P2, USER_DATA_ENCRYPTION_BIT_PK, USER_DATA_ENCRYPTION_BIT_R1,
    USER_DATA_ENCRYPTION_BIT_R2, USER_DATA_ENCRYPTION_BIT_U, USER_DATA_ENCRYPTION_CONFIGS,
};
use lib::core::threshold::user_data_encryption::UserDataEncryption;
use lib::math::commitments::compute_ciphertext_commitment;
use lib::math::polynomial::Polynomial;

mod constants;
mod ciphertext_addition;
use ciphertext_addition::CiphertextAddition;
mod ecdsa;
use ecdsa::{address_to_field, derive_address, validate_signature};
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
/// The circuit verifies all inputs and returns the appropriate ciphertext commitment.
/// Specifically, it returns a Field commitment depending on the case:
/// - **Actual vote**: `ct_commitment` - commitment to the new vote ciphertext
/// - **Mask vote (first vote)**: `ct_commitment` - commitment to the zero ciphertext
/// - **Mask vote (updating slot)**: `sum_ct_commitment` - commitment to the sum of previous votes in slot
///
fn main(
    // Ciphertext Addition Section
    prev_ct0is: [Polynomial<N>; L],
    prev_ct1is: [Polynomial<N>; L],
    prev_ct_commitment: pub Field,
    sum_ct0is: [Polynomial<N>; L],
    sum_ct1is: [Polynomial<N>; L],
    sum_r0is: [Polynomial<N>; L],
    sum_r1is: [Polynomial<N>; L],
    // Greco Section.
    pk_commitment: pub Field,
    pk0is: [Polynomial<N>; L],
    pk1is: [Polynomial<N>; L],
    ct0is: [Polynomial<N>; L],
    ct1is: [Polynomial<N>; L],
    u: Polynomial<N>,
    e0: Polynomial<N>,
    e0is: [Polynomial<N>; L],
    e0_quotients: [Polynomial<N>; L],
    e1: Polynomial<N>,
    k1: Polynomial<N>,
    r1is: [Polynomial<(2 * N) - 1>; L],
    r2is: [Polynomial<N - 1>; L],
    p1is: [Polynomial<(2 * N) - 1>; L],
    p2is: [Polynomial<N - 1>; L],
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
    // Other inputs.
    slot_address: pub Field,
    balance: Field,
    is_first_vote: pub bool,
    is_mask_vote: bool,
    num_options: pub u32,
) -> pub Field {
    // ============================================================================
    // STEP 1: Eligibility - Merkle Tree Proof
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
    // STEP 2: BFV Encryption Verification (GRECO)
    // ============================================================================
    // Verify that the ciphertext (ct0is, ct1is) is a valid encryption of the
    // plaintext k1 under the public key (pk0is, pk1is). This ensures the ciphertext
    // is correctly formed.
    //
    // This check applies to BOTH cases:
    // - For actual votes: verifies the vote ciphertext is correctly formed
    // - For mask votes: verifies the zero vote ciphertext is correctly formed
    let user_data_encryption: UserDataEncryption<N, L, USER_DATA_ENCRYPTION_BIT_PK, USER_DATA_ENCRYPTION_BIT_CT, USER_DATA_ENCRYPTION_BIT_U, USER_DATA_ENCRYPTION_BIT_E0, USER_DATA_ENCRYPTION_BIT_E1, USER_DATA_ENCRYPTION_BIT_K, USER_DATA_ENCRYPTION_BIT_R1, USER_DATA_ENCRYPTION_BIT_R2, USER_DATA_ENCRYPTION_BIT_P1, USER_DATA_ENCRYPTION_BIT_P2> = UserDataEncryption::new(
        USER_DATA_ENCRYPTION_CONFIGS,
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

    assert(user_data_encryption.execute());

    // ============================================================================
    // STEP 3: Vote Type Detection and Return Logic
    // ============================================================================
    // The circuit branches into two cases based on the is_mask_vote flag:
    //
    // CASE 1: ACTUAL VOTE (is_mask_vote == false)
    //   - This is an eligible voter casting a vote
    //   - Verify vote amount <= balance
    //   - Validate signature over the hashed message (voter authenticates themselves)
    //   - Verify address matches slot address
    //   - Return new vote ciphertext commitment (ct_commitment)
    //
    // CASE 2: MASK VOTE (is_mask_vote == true)
    //   - This is anyone submitting a zero vote to mask slot activity
    //   - Verify vote is zero (k1 must be zero)
    //   - If first vote in slot: return zero ciphertext commitment (ct_commitment)
    //   - If updating slot:
    //     * Verify that the sum ciphertext correctly represents the homomorphic addition
    //       of a zero vote to the previous ciphertext without decrypting
    //     * Mask votes add zero to the previous ciphertext, creating a different ciphertext
    //       with the same plaintext as the previous ciphertext
    //     * Generate commitments for prev_ct and sum_ct (ct_commitment is generated before)
    //     * Verify prev_ct_commitment matches the actual polynomials to ensure the prover
    //       hasn't tampered with the previous ciphertext
    //     * Verify ciphertext addition using commitments and Fiat-Shamir transform with
    //       Schwartz-Zippel lemma verification
    //     * Return sum ciphertext commitment (sum_ct_commitment)

    // Generate the vote ciphertext commitment.
    let ct_commitment = compute_ciphertext_commitment::<512, 2, 36>(ct0is, ct1is);

    if is_mask_vote == false {
        check_coefficient_values_with_balance(k1, Q_MOD_T_CENTERED, balance, num_options);
        validate_signature(hashed_message, public_key_x, public_key_y, signature);

        let voter_address = address_to_field(derive_address(public_key_x, public_key_y));
        assert(slot_address == voter_address);

        ct_commitment
    } else {
        check_coefficient_zero(k1, num_options);

        if is_first_vote {
            ct_commitment
        } else {
            let _prev_ct_commitment =
                compute_ciphertext_commitment::<512, 2, 36>(prev_ct0is, prev_ct1is);
            let sum_ct_commitment =
                compute_ciphertext_commitment::<512, 2, 36>(sum_ct0is, sum_ct1is);

            let ct_add: CiphertextAddition<512, 2, 36, 36, 36> = CiphertextAddition::new(
                USER_DATA_ENCRYPTION_CONFIGS,
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

            assert(prev_ct_commitment == _prev_ct_commitment);
            assert(ct_add.execute());

            sum_ct_commitment
        }
    }
}
