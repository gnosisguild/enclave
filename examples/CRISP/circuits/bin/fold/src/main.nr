// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};
use keccak256::keccak256;

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
) -> pub (Field, [u8; 32]) {
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

    // Hash all verification key-related values: ud_key_hash, crisp_key_hash, ct0_key_hash, ct1_key_hash
    let ud_bytes: [u8; 32] = user_data_encryption_key_hash.to_be_bytes();
    let crisp_bytes: [u8; 32] = crisp_key_hash.to_be_bytes();
    let ct0_bytes: [u8; 32] = user_data_encryption_public_inputs[0].to_be_bytes();
    let ct1_bytes: [u8; 32] = user_data_encryption_public_inputs[1].to_be_bytes();
    let mut combined: [u8; 128] = [0; 128];
    for i in 0..32 {
        combined[i] = ud_bytes[i];
        combined[i + 32] = crisp_bytes[i];
        combined[i + 64] = ct0_bytes[i];
        combined[i + 96] = ct1_bytes[i];
    }
    let key_hash: [u8; 32] = keccak256(combined, 128);

    // Return the verification key hash and pk_commitment from the user data encryption proof (this will
    // be verified in the crisp contract).
    (user_data_encryption_public_inputs[2], key_hash)
}
