// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};
use enclave_lib::math::commitments::{compute_commitment, DS_CIPHERTEXT, DS_PK_AGGREGATION};

fn main(
    // User Data Encryption ct0 Section.
    ct0_verification_key: UltraHonkVerificationKey,
    ct0_proof: UltraHonkProof,
    ct0_public_inputs: [Field; 4], // pk0_commitment, ct0_commitment, k1_commitment, u_commitment
    ct0_key_hash: pub Field,
    // User Data Encryption ct1 Section.
    ct1_verification_key: UltraHonkVerificationKey,
    ct1_proof: UltraHonkProof,
    ct1_public_inputs: [Field; 3], // pk1_commitment, ct1_commitment, u_commitment
    ct1_key_hash: pub Field,
    // Crisp Section.
    crisp_verification_key: UltraHonkVerificationKey,
    crisp_proof: UltraHonkProof,
    prev_ct_commitment: pub Field,
    merkle_root: pub Field,
    slot_address: pub Field,
    is_first_vote: pub bool,
    num_options: pub u32,
    final_ct_commitment: pub Field,
    ct_commitment: Field,
    k1_commitment: Field,
    crisp_key_hash: pub Field,
) -> pub Field {
    verify_honk_proof_non_zk(
        ct0_verification_key,
        ct0_proof,
        ct0_public_inputs,
        ct0_key_hash,
    );
    verify_honk_proof_non_zk(
        ct1_verification_key,
        ct1_proof,
        ct1_public_inputs,
        ct1_key_hash,
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

    // Verify that the u_commitment from the ct0 proof is the same as the u_commitment from the ct1 proof.
    assert(ct0_public_inputs[3] == ct1_public_inputs[2]);

    let mut ct_inputs = Vec::new();
    ct_inputs.push(ct0_public_inputs[1]);
    ct_inputs.push(ct1_public_inputs[1]);
    let computed_ct_commitment = compute_commitment(ct_inputs, DS_CIPHERTEXT);

    // Verify that the ct_commitment from the crisp proof matches the one computed from ct0 and ct1.
    assert(ct_commitment == computed_ct_commitment);

    // Verify that the k1_commitment from the crisp proof matches the one computed from ct0 and ct1.
    assert(k1_commitment == ct0_public_inputs[2]);

    // Computes the public key aggregation commitment.
    let mut pk_inputs = Vec::new();
    pk_inputs.push(ct0_public_inputs[0]);
    pk_inputs.push(ct1_public_inputs[0]);

    compute_commitment(pk_inputs, DS_PK_AGGREGATION)
}
