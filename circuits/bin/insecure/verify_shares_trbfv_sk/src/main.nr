// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::insecure::trbfv::{
    L, N, VERIFY_SHARES_BIT_SECRET_SK, VERIFY_SHARES_BIT_SHARE, VERIFY_SHARES_CONFIGS_SK,
};
use lib::core::trbfv_verify_shares::VerifySharesSk;
use lib::math::polynomial::Polynomial;

/// Number of parties.
pub global N_PARTIES: u32 = 5;
/// Threshold.
pub global T: u32 = 2;

fn main(
    expected_secret_commitment: pub Field,
    secret_sk: Polynomial<N>,
    y: [[[Field; N_PARTIES + 1]; L]; N],
    h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
) -> pub [[Field; L]; N_PARTIES] {
    let verify_shares: VerifySharesSk<N, L, N_PARTIES, T, VERIFY_SHARES_BIT_SECRET_SK, VERIFY_SHARES_BIT_SHARE> = VerifySharesSk::new(
        VERIFY_SHARES_CONFIGS_SK,
        expected_secret_commitment,
        secret_sk,
        y,
        h,
    );

    verify_shares.verify()
}
