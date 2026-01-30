// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::default::dkg::{DEC_BFV_BIT_MSG_E_SM, L_THRESHOLD, N};
use lib::configs::default::H;
use lib::core::bfv_dec::BfvDecCommitVerify;
use lib::math::polynomial::Polynomial;

fn main(
    expected_commitments: pub [[Field; L_THRESHOLD]; H],
    decrypted_shares: [[Polynomial<N>; L_THRESHOLD]; H],
) -> pub Field {
    let circuit: BfvDecCommitVerify<N, L_THRESHOLD, H, DEC_BFV_BIT_MSG_E_SM> =
        BfvDecCommitVerify::new(expected_commitments, decrypted_shares);

    circuit.verify()
}
