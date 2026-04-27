// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Decryption aggregator: non-ZK `c6_fold` (folded C6 column) + non-ZK `decrypted_shares_aggregation`
//! (C7, both `noir-recursive-no-zk`). The final `bb prove -t evm` proof of this circuit is what the
//! contract verifies on-chain.

use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};
use lib::configs::default::MAX_MSG_NON_ZERO_COEFFS;
use lib::configs::default::T;
use lib::math::commitments::compute_vk_hash;

/// Must match `c6_fold::C6_FOLD_PUBLIC_LEN`: pub params + flattened `(out_sk, out_esm, out_ct, out_d)`.
pub global C6_FOLD_PUBLIC_LEN: u32 = 4 + (4 * (T + 1));

pub global C6_FOLD_PREFIX_LEN: u32 = C6_FOLD_PUBLIC_LEN - (4 * (T + 1));

/// Column offsets inside the `c6_fold` flattened tail (after `C6_FOLD_PREFIX_LEN`):
/// `(out_sk, out_esm, out_ct, out_d)`, each `C6_FOLD_COL_WIDTH = T + 1` long.
pub global C6_FOLD_COL_WIDTH: u32 = T + 1;
pub global C6_FOLD_SK_COL_OFFSET: u32 = 0;
pub global C6_FOLD_ESM_COL_OFFSET: u32 = C6_FOLD_COL_WIDTH;
pub global C6_FOLD_CT_COL_OFFSET: u32 = 2 * C6_FOLD_COL_WIDTH;
pub global C6_FOLD_D_COL_OFFSET: u32 = 3 * C6_FOLD_COL_WIDTH;

/// `c7_public` layout: `[expected_d_commitments | party_ids | message]`.
pub global C7_D_COMMITMENT_OFFSET: u32 = 0;
pub global C7_PARTY_ID_OFFSET: u32 = T + 1;
pub global C7_MESSAGE_OFFSET: u32 = 2 * (T + 1);

pub global C7_PUBLIC_LEN: u32 = (2 * (T + 1)) + MAX_MSG_NON_ZERO_COEFFS;

fn main(
    c6_fold_vk: UltraHonkVerificationKey,
    c6_fold_proof: UltraHonkProof,
    c6_fold_public: [Field; C6_FOLD_PUBLIC_LEN],
    c7_vk: UltraHonkVerificationKey,
    c7_proof: UltraHonkProof,
    c7_public: [Field; C7_PUBLIC_LEN],
    c6_fold_key_hash: pub Field,
    c7_key_hash: pub Field,
) -> pub (Field, [Field; T + 1], [Field; T + 1], [Field; T + 1], [Field; MAX_MSG_NON_ZERO_COEFFS]) {
    verify_honk_proof_non_zk(c6_fold_vk, c6_fold_proof, c6_fold_public, c6_fold_key_hash);
    verify_honk_proof_non_zk(c7_vk, c7_proof, c7_public, c7_key_hash);

    // Cross-circuit C6 to C7: each folded `d_commitment` column entry must match `c7_public[i]`
    // (leading `T + 1` words of `c7_public`).
    for i in 0..(T + 1) {
        assert(
            c6_fold_public[C6_FOLD_PREFIX_LEN + C6_FOLD_D_COL_OFFSET + i]
                == c7_public[C7_D_COMMITMENT_OFFSET + i],
        );
    }

    let mut vk_hashes = Vec::new();
    vk_hashes.push(c6_fold_key_hash);
    vk_hashes.push(c7_key_hash);
    let key_hash = compute_vk_hash(vk_hashes);

    let mut party_ids: [Field; T + 1] = [0; T + 1];
    let mut expected_sk: [Field; T + 1] = [0; T + 1];
    let mut expected_esm: [Field; T + 1] = [0; T + 1];
    for i in 0..(T + 1) {
        party_ids[i] = c7_public[C7_PARTY_ID_OFFSET + i];
        expected_sk[i] = c6_fold_public[C6_FOLD_PREFIX_LEN + C6_FOLD_SK_COL_OFFSET + i];
        expected_esm[i] = c6_fold_public[C6_FOLD_PREFIX_LEN + C6_FOLD_ESM_COL_OFFSET + i];
    }

    let mut message: [Field; MAX_MSG_NON_ZERO_COEFFS] = [0; MAX_MSG_NON_ZERO_COEFFS];
    for k in 0..MAX_MSG_NON_ZERO_COEFFS {
        message[k] = c7_public[C7_MESSAGE_OFFSET + k];
    }

    (key_hash, party_ids, expected_sk, expected_esm, message)
}
