// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::default::dkg::{L_THRESHOLD, N, SHARE_COMPUTATION_SK_BIT_SECRET};
use lib::configs::default::{N_PARTIES, T};
use lib::core::dkg::share_computation::base::SecretKeyShareComputationBase;
use lib::math::polynomial::Polynomial;

fn main(
    expected_secret_commitment: pub Field,
    sk_secret: Polynomial<N>,
    // y is public so wrapper can enforce consistency with chunk circuits
    y: pub [[[Field; N_PARTIES + 1]; L_THRESHOLD]; N],
) -> pub [[Field; L_THRESHOLD]; N_PARTIES] {
    let circuit: SecretKeyShareComputationBase<N, L_THRESHOLD, N_PARTIES, T, SHARE_COMPUTATION_SK_BIT_SECRET> =
        SecretKeyShareComputationBase::new(expected_secret_commitment, sk_secret, y);
    circuit.execute()
}
