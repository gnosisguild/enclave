// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::default::threshold::{
    L, N, THRESHOLD_SHARE_DECRYPTION_BIT_CT, THRESHOLD_SHARE_DECRYPTION_BIT_D,
    THRESHOLD_SHARE_DECRYPTION_BIT_E_SM, THRESHOLD_SHARE_DECRYPTION_BIT_R1,
    THRESHOLD_SHARE_DECRYPTION_BIT_R2, THRESHOLD_SHARE_DECRYPTION_BIT_SK,
    THRESHOLD_SHARE_DECRYPTION_CONFIGS,
};
use lib::core::threshold::share_decryption::ShareDecryption;
use lib::math::polynomial::Polynomial;

fn main(
    expected_sk_commitment: pub Field,
    expected_e_sm_commitment: pub Field,
    ct0: pub [Polynomial<N>; L],
    ct1: pub [Polynomial<N>; L],
    sk: [Polynomial<N>; L],
    e_sm: [Polynomial<N>; L],
    r1: [Polynomial<(2 * N) - 1>; L],
    r2: [Polynomial<N - 1>; L],
    d: [Polynomial<N>; L],
) {
    let share_decryption: ShareDecryption<N, L, THRESHOLD_SHARE_DECRYPTION_BIT_CT, THRESHOLD_SHARE_DECRYPTION_BIT_SK, THRESHOLD_SHARE_DECRYPTION_BIT_E_SM, THRESHOLD_SHARE_DECRYPTION_BIT_R1, THRESHOLD_SHARE_DECRYPTION_BIT_R2, THRESHOLD_SHARE_DECRYPTION_BIT_D> = ShareDecryption::new(
        THRESHOLD_SHARE_DECRYPTION_CONFIGS,
        expected_sk_commitment,
        expected_e_sm_commitment,
        ct0,
        ct1,
        sk,
        e_sm,
        r1,
        r2,
        d,
    );
    share_decryption.execute()
}
