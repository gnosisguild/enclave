// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::production::trbfv::{
    L, N, VERIFY_SHARES_BIT_SECRET_E_SM, VERIFY_SHARES_BIT_SHARE, VERIFY_SHARES_CONFIGS_E_SM,
};
use lib::core::trbfv_verify_shares::VerifySharesEsm;
use lib::math::polynomial::Polynomial;

/// Number of parties.
pub global N_PARTIES: u32 = 5;
/// Threshold.
pub global T: u32 = 2;
/// Parity check matrix for each modulus.
/// H[modulus_idx][row][col] where size is [L][N_PARTIES-T][N_PARTIES+1]
pub global H: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L] = [
    [
        [2251799822204928, 3, 2251799822204926, 1, 0, 0],
        [2251799822204926, 8, 2251799822204923, 0, 1, 0],
        [2251799822204923, 15, 2251799822204919, 0, 0, 1],
    ],
    [
        [4503599627763712, 3, 4503599627763710, 1, 0, 0],
        [4503599627763710, 8, 4503599627763707, 0, 1, 0],
        [4503599627763707, 15, 4503599627763703, 0, 0, 1],
    ],
    [
        [4503599631433728, 3, 4503599631433726, 1, 0, 0],
        [4503599631433726, 8, 4503599631433723, 0, 1, 0],
        [4503599631433723, 15, 4503599631433719, 0, 0, 1],
    ],
    [
        [4503599634579456, 3, 4503599634579454, 1, 0, 0],
        [4503599634579454, 8, 4503599634579451, 0, 1, 0],
        [4503599634579451, 15, 4503599634579447, 0, 0, 1],
    ],
];

fn main(
    expected_secret_commitment: pub Field,
    secret_e_sm: [Polynomial<N>; L],
    y: [[[Field; N_PARTIES + 1]; L]; N],
) -> pub [[Field; L]; N_PARTIES] {
    let verify_shares: VerifySharesEsm<N, L, N_PARTIES, T, VERIFY_SHARES_BIT_SECRET_E_SM, VERIFY_SHARES_BIT_SHARE> = VerifySharesEsm::new(
        VERIFY_SHARES_CONFIGS_E_SM,
        expected_secret_commitment,
        secret_e_sm,
        y,
        H,
    );

    verify_shares.verify()
}
