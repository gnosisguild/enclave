// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::production::trbfv::{
    DEC_SHARES_BIT_CT, DEC_SHARES_BIT_D, DEC_SHARES_BIT_E, DEC_SHARES_BIT_R1, DEC_SHARES_BIT_R2,
    DEC_SHARES_BIT_S, DEC_SHARES_CONFIGS, L, N,
};
use lib::core::trbfv_dec_share::DecryptionShare;
use lib::math::polynomial::Polynomial;

/// Number of parties.
pub global N_PARTIES: u32 = 5;
/// Threshold.
pub global T: u32 = 2;

fn main(
    expected_s_commitment: pub Field,
    expected_e_commitment: pub Field,
    c_0: [Polynomial<N>; L],
    c_1: [Polynomial<N>; L],
    s: [Polynomial<N>; L],
    e: [Polynomial<N>; L],
    r_1: [Polynomial<(2 * N) - 1>; L],
    r_2: [Polynomial<N - 1>; L],
    d: [Polynomial<N>; L],
) {
    let dec_share: DecryptionShare<N, L, DEC_SHARES_BIT_CT, DEC_SHARES_BIT_S, DEC_SHARES_BIT_E, DEC_SHARES_BIT_R1, DEC_SHARES_BIT_R2, DEC_SHARES_BIT_D> = DecryptionShare::new(
        DEC_SHARES_CONFIGS,
        expected_s_commitment,
        expected_e_commitment,
        c_0,
        c_1,
        s,
        e,
        r_1,
        r_2,
        d,
    );

    dec_share.verify()
}
