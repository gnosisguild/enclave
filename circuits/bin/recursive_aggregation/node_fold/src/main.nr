// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Per-node fold: C0, C1, `c2ab_fold`, `c3ab_fold`, `c4ab_fold` with same-party assertions.

use bb_proof_verification::{
    UltraHonkProof, UltraHonkVerificationKey, UltraHonkZKProof, verify_honk_proof,
    verify_honk_proof_non_zk,
};
use lib::configs::default::dkg::L_THRESHOLD;
use lib::configs::default::{H, N_PARTIES};
use lib::math::commitments::compute_vk_hash;

pub global C2_PUBLIC_LEN: u32 = 1 + (N_PARTIES * L_THRESHOLD);
pub global C2AB_FOLD_PUBLIC_LEN: u32 = 3 + (2 * C2_PUBLIC_LEN);
/// One slot per (party, threshold-modulus) pair - must match `c3ab_fold::C3AB_FOLD_PUBLIC_LEN`.
pub global C3_SLOTS: u32 = N_PARTIES * L_THRESHOLD;
pub global C3AB_FOLD_PUBLIC_LEN: u32 = 2 + 1 + (6 * C3_SLOTS);
pub global C4_PUBLIC_LEN: u32 = (H * L_THRESHOLD) + 1;
pub global C4AB_FOLD_PUBLIC_LEN: u32 = 3 + (2 * C4_PUBLIC_LEN);

/// Total public parameters + outputs for `node_fold`.
pub global NODE_FOLD_PUBLIC_LEN: u32 = 11 + N_PARTIES + (2 * (N_PARTIES + H) * L_THRESHOLD);

fn main(
    c0_vk: UltraHonkVerificationKey,
    c0_proof: UltraHonkZKProof,
    c0_public: [Field; 1],
    c1_vk: UltraHonkVerificationKey,
    c1_proof: UltraHonkZKProof,
    c1_public: [Field; 3],
    c2ab_vk: UltraHonkVerificationKey,
    c2ab_proof: UltraHonkProof,
    c2ab_public: [Field; C2AB_FOLD_PUBLIC_LEN],
    c3ab_vk: UltraHonkVerificationKey,
    c3ab_proof: UltraHonkProof,
    c3ab_public: [Field; C3AB_FOLD_PUBLIC_LEN],
    c4ab_vk: UltraHonkVerificationKey,
    c4ab_proof: UltraHonkProof,
    c4ab_public: [Field; C4AB_FOLD_PUBLIC_LEN],
    _party_id: pub Field,
    c0_key_hash: pub Field,
    c1_key_hash: pub Field,
    c2ab_key_hash: pub Field,
    c3ab_key_hash: pub Field,
    c4ab_key_hash: pub Field,
    ) -> pub (Field, Field, Field, [Field; N_PARTIES], [[Field; L_THRESHOLD]; N_PARTIES], [[Field; L_THRESHOLD]; N_PARTIES], [[Field; L_THRESHOLD]; H], [[Field; L_THRESHOLD]; H], Field, Field) {
    verify_honk_proof(c0_vk, c0_proof, c0_public, c0_key_hash);
    verify_honk_proof(c1_vk, c1_proof, c1_public, c1_key_hash);
    verify_honk_proof_non_zk(c2ab_vk, c2ab_proof, c2ab_public, c2ab_key_hash);
    verify_honk_proof_non_zk(c3ab_vk, c3ab_proof, c3ab_public, c3ab_key_hash);
    verify_honk_proof_non_zk(c4ab_vk, c4ab_proof, c4ab_public, c4ab_key_hash);

    let mut vk_hashes = Vec::new();
    vk_hashes.push(c0_key_hash);
    vk_hashes.push(c1_key_hash);
    vk_hashes.push(c2ab_key_hash);
    vk_hashes.push(c3ab_key_hash);
    vk_hashes.push(c4ab_key_hash);
    let key_hash = compute_vk_hash(vk_hashes);

    let c0_pk = c0_public[0];
    let c1_pk = c1_public[1];

    // Same-party C1 to C2: `c2ab_public` lays out two full C2 public vectors after a 3-field
    // prefix (`C2AB_FOLD_PUBLIC_LEN` = 3 + 2 * `C2_PUBLIC_LEN`). For each inner C2 block, index 0
    // is `expected_secret_commitment`; these equal C1's sk- and e_sm-side public inputs [0] and [2].
    assert(c1_public[0] == c2ab_public[3]);
    assert(c1_public[2] == c2ab_public[10]);

    // Same-party C3a and C3b: pk_a[j*L+l] and pk_b[j*L+l] must match for every (party, modulus)
    // slot - recipient public key must be consistent across both folded C3 chains.
    for j in 0..N_PARTIES {
        for l in 0..L_THRESHOLD {
            assert(
                c3ab_public[3 + j * L_THRESHOLD + l]
                    == c3ab_public[3 + 3 * C3_SLOTS + j * L_THRESHOLD + l],
            );
        }
    }

    // C2 to C3: all (party, modulus) slots must be bound - msg_a[j*L+l] == c2a_shares[j][l]
    // and msg_b[j*L+l] == c2b_shares[j][l].
    // c3ab layout: [0..1] key hashes, [2] combined hash, [3..3+C3_SLOTS-1] pk_a,
    //              [3+C3_SLOTS..3+2*C3_SLOTS-1] msg_a, ..., [3+4*C3_SLOTS..] msg_b.
    // c2ab layout: [0..2] key hashes, [3] c2a_expected, [4..4+N_PARTIES*L-1] c2a_shares,
    //              [4+N_PARTIES*L] c2b_expected, [5+N_PARTIES*L..] c2b_shares.
    for j in 0..N_PARTIES {
        for l in 0..L_THRESHOLD {
            let slot = j * L_THRESHOLD + l;
            assert(c2ab_public[4 + slot] == c3ab_public[3 + C3_SLOTS + slot]);
            assert(
                c2ab_public[5 + N_PARTIES * L_THRESHOLD + slot]
                    == c3ab_public[3 + 4 * C3_SLOTS + slot],
            );
        }
    }

    let mut c3_pk: [Field; N_PARTIES] = [0; N_PARTIES];
    for j in 0..N_PARTIES {
        // pk is per-party (same DKG key across all moduli of the same recipient): use slot j*L+0.
        c3_pk[j] = c3ab_public[3 + j * L_THRESHOLD];
    }

    let mut c2a: [[Field; L_THRESHOLD]; N_PARTIES] = [[0; L_THRESHOLD]; N_PARTIES];
    let mut c2b: [[Field; L_THRESHOLD]; N_PARTIES] = [[0; L_THRESHOLD]; N_PARTIES];
    for j in 0..N_PARTIES {
        for l in 0..L_THRESHOLD {
            c2a[j][l] = c2ab_public[4 + j * L_THRESHOLD + l];
            c2b[j][l] = c2ab_public[11 + j * L_THRESHOLD + l];
        }
    }

    let mut c4a_exp: [[Field; L_THRESHOLD]; H] = [[0; L_THRESHOLD]; H];
    let mut c4b_exp: [[Field; L_THRESHOLD]; H] = [[0; L_THRESHOLD]; H];
    for h in 0..H {
        for l in 0..L_THRESHOLD {
            c4a_exp[h][l] = c4ab_public[3 + h * L_THRESHOLD + l];
            c4b_exp[h][l] = c4ab_public[10 + h * L_THRESHOLD + l];
        }
    }
    let sk_agg = c4ab_public[9];
    let esm_agg = c4ab_public[16];

    (key_hash, c0_pk, c1_pk, c3_pk, c2a, c2b, c4a_exp, c4b_exp, sk_agg, esm_agg)
}
