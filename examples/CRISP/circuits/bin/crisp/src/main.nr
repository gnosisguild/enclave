// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use bb_proof_verification::{UltraHonkVerificationKey, UltraHonkZKProof, verify_ultrahonkzk_proof};
use crisp_lib::ciphertext_addition::CiphertextAddition;
use crisp_lib::commitments::{compute_ct_commitment, compute_k1_commitment};
use crisp_lib::ecdsa::{address_to_field, derive_address, validate_signature};
use crisp_lib::merkle_tree::get_merkle_root;
use crisp_lib::utils::{check_coefficient_values_with_balance, check_coefficient_zero};
use pvss_lib::configs::default::threshold::{
    L, N, Q_MOD_T_CENTERED, USER_DATA_ENCRYPTION_BIT_CT, USER_DATA_ENCRYPTION_BIT_K,
    USER_DATA_ENCRYPTION_BIT_R1, USER_DATA_ENCRYPTION_BIT_R2, USER_DATA_ENCRYPTION_CONFIGS,
};
use pvss_lib::math::commitments::compute_ciphertext_commitment;
use pvss_lib::math::polynomial::Polynomial;

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
    greco_verification_key: UltraHonkVerificationKey,
    greco_key_hash: Field,
    greco_proof: UltraHonkZKProof,
    ct0is: [Polynomial<N>; L],
    ct1is: [Polynomial<N>; L],
    k1: Polynomial<N>,
    pk_commitment: pub Field,
    k1_commitment: Field,
    ct_commitment: Field,
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
    // Verify that the ciphertext (ct0is, ct1is) is a valid BFV encryption of the
    // plaintext k1 under the public key represented by pk_commitment. This ensures
    // the ciphertext is correctly formed and maintains the encryption relationship.
    //
    // The verification uses a recursive zero-knowledge proof (GRECO) to prove the encryption
    // relationship without revealing the underlying values. The recursive proof demonstrates:
    // - The ciphertext (ct0is, ct1is) correctly encrypts plaintext k1
    // - The encryption uses the public key committed to in pk_commitment
    // - The verification key hash matches the expected greco_key_hash
    //
    // After verifying the proof, we check that the provided commitments match the
    // actual computed commitments to ensure consistency:
    // - k1_commitment must match the commitment computed from k1
    // - ct_commitment must match the commitment computed from (ct0is, ct1is)
    //
    // This check applies to BOTH cases:
    // - For actual votes: verifies the vote ciphertext is correctly formed
    // - For mask votes: verifies the zero vote ciphertext is correctly formed
    verify_ultrahonkzk_proof(
        greco_verification_key,
        greco_proof,
        [pk_commitment, k1_commitment, ct_commitment],
        greco_key_hash,
    );
    assert(k1_commitment == compute_k1_commitment::<N, USER_DATA_ENCRYPTION_BIT_K>(k1));
    assert(
        ct_commitment == compute_ct_commitment::<N, L, USER_DATA_ENCRYPTION_BIT_CT>(ct0is, ct1is),
    );

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
    let ct_commitment =
        compute_ciphertext_commitment::<N, L, USER_DATA_ENCRYPTION_BIT_CT>(ct0is, ct1is);

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
            let _prev_ct_commitment = compute_ciphertext_commitment::<N, L, USER_DATA_ENCRYPTION_BIT_CT>(
                prev_ct0is,
                prev_ct1is,
            );
            let sum_ct_commitment = compute_ciphertext_commitment::<N, L, USER_DATA_ENCRYPTION_BIT_CT>(
                sum_ct0is,
                sum_ct1is,
            );

            let ct_add: CiphertextAddition<N, L, USER_DATA_ENCRYPTION_BIT_CT, USER_DATA_ENCRYPTION_BIT_CT, USER_DATA_ENCRYPTION_BIT_CT> = CiphertextAddition::new(
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
