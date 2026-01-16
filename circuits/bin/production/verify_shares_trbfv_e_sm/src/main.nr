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

fn main(
    expected_secret_commitment: Field,
    secret_e_sm: [Polynomial<N>; L],
    y: [[[Field; N_PARTIES + 1]; L]; N],
    h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
) -> pub [[Field; L]; N_PARTIES] {
    let verify_shares: VerifySharesEsm<N, L, N_PARTIES, T, VERIFY_SHARES_BIT_SECRET_E_SM, VERIFY_SHARES_BIT_SHARE> = VerifySharesEsm::new(
        VERIFY_SHARES_CONFIGS_E_SM,
        expected_secret_commitment,
        secret_e_sm,
        y,
        h,
    );

    verify_shares.verify()
}
