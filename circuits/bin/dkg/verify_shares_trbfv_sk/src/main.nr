// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::default::dkg::{
    L_THRESHOLD, N, PARITY_MATRIX, VERIFY_SHARES_BIT_SECRET_SK, VERIFY_SHARES_BIT_SHARE,
    VERIFY_SHARES_CONFIGS_SK,
};
use lib::configs::default::{N_PARTIES, T};
use lib::core::trbfv_verify_shares::VerifySharesSk;
use lib::math::polynomial::Polynomial;

fn main(
    expected_secret_commitment: pub Field,
    secret_sk: Polynomial<N>,
    y: [[[Field; N_PARTIES + 1]; L_THRESHOLD]; N],
) -> pub [[Field; L_THRESHOLD]; N_PARTIES] {
    let verify_shares: VerifySharesSk<N, L_THRESHOLD, N_PARTIES, T, VERIFY_SHARES_BIT_SECRET_SK, VERIFY_SHARES_BIT_SHARE> = VerifySharesSk::new(
        VERIFY_SHARES_CONFIGS_SK,
        expected_secret_commitment,
        secret_sk,
        y,
        PARITY_MATRIX,
    );

    verify_shares.verify()
}
