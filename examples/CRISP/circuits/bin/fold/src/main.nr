// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};
use enclave_lib::math::commitments::compute_vk_hash;

fn main(
    // User Data Encryption Section.
    user_data_encryption_verification_key: UltraHonkVerificationKey,
    user_data_encryption_proof: UltraHonkProof,
    user_data_encryption_public_inputs: [Field; 5], // ct0_key_hash, ct1_key_hash, pk_commitment, ct_commitment, k1_commitment
    user_data_encryption_key_hash: Field,
    // Crisp Section.
    crisp_verification_key: UltraHonkVerificationKey,
    crisp_proof: UltraHonkProof,
    crisp_key_hash: Field,
    prev_ct_commitment: pub Field,
    merkle_root: pub Field,
    slot_address: pub Field,
    is_first_vote: pub bool,
    num_options: pub u32,
    final_ct_commitment: pub Field,
    ct_commitment: Field,
    k1_commitment: Field,
) -> pub (Field, Field) {
    verify_honk_proof_non_zk(
        user_data_encryption_verification_key,
        user_data_encryption_proof,
        user_data_encryption_public_inputs,
        user_data_encryption_key_hash,
    );
    verify_honk_proof_non_zk(
        crisp_verification_key,
        crisp_proof,
        [
            prev_ct_commitment,
            merkle_root,
            slot_address,
            if is_first_vote { 1 } else { 0 },
            num_options as Field,
            final_ct_commitment,
            ct_commitment,
            k1_commitment,
        ],
        crisp_key_hash,
    );

    // Verify that the ct_commitment from the crisp proof matches the one computed from user data encryption.
    assert(ct_commitment == user_data_encryption_public_inputs[3]);

    // Verify that the k1_commitment from the crisp proof matches the one computed from user data encryption.
    assert(k1_commitment == user_data_encryption_public_inputs[4]);

    // Hash the full VK chain: key hashes from both proofs (user_data_encryption, crisp, ct0, ct1) so the verifier can
    // check the entire proof genealogy.
    let mut vk_hashes = Vec::new();
    vk_hashes.push(user_data_encryption_key_hash);
    vk_hashes.push(crisp_key_hash);
    vk_hashes.push(user_data_encryption_public_inputs[0]); // ct0_key_hash
    vk_hashes.push(user_data_encryption_public_inputs[1]); // ct1_key_hash
    let key_hash = compute_vk_hash(vk_hashes);

    // Return pk_commitment from user data encryption proof and the combined VK hash (verified in crisp contract).
    (user_data_encryption_public_inputs[2], key_hash)
}
