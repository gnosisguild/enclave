// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::default::dkg::{
    L_THRESHOLD, N, QIS_THRESHOLD, SHARE_COMPUTATION_PRF_KEY_BITS, SHARE_DECRYPTION_BIT_AGG,
    SHARE_DECRYPTION_BIT_MSG,
};
use lib::configs::default::{H, N_PARTIES};
use lib::core::dkg::share_decryption::ShareDecryption;
use lib::math::polynomial::Polynomial;

fn main(
    expected_commitments: pub [[Field; L_THRESHOLD]; H],
    recipient_party_idx: pub u32,
    prf_key_commitments: pub [Field; H],
    prf_keys: [[bool; SHARE_COMPUTATION_PRF_KEY_BITS]; H],
    decrypted_shares: [[Polynomial<N>; L_THRESHOLD]; H],
) -> pub Field {
    assert(recipient_party_idx < N_PARTIES);
    let share_decryption: ShareDecryption<N, L_THRESHOLD, H, SHARE_DECRYPTION_BIT_MSG, SHARE_DECRYPTION_BIT_AGG> =
        ShareDecryption::new(expected_commitments, decrypted_shares);

    let (aggregate, keys) =
        share_decryption.execute_with_prf(QIS_THRESHOLD, recipient_party_idx, prf_keys);
    assert(keys == prf_key_commitments, "PRF key commitment mismatch");
    aggregate
}
