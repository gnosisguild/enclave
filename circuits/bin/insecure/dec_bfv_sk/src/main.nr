// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::insecure::bfv::{DEC_BFV_BIT_MSG_SK, L_TRBFV, N};
use lib::core::bfv_dec::BfvDecCommitVerify;
use lib::math::polynomial::Polynomial;

/// Number of honest parties.
pub global H: u32 = 5;

fn main(
    expected_commitments: [[Field; L_TRBFV]; H],
    decrypted_shares: [[Polynomial<N>; L_TRBFV]; H],
) -> pub Field {
    let circuit: BfvDecCommitVerify<N, L_TRBFV, H, DEC_BFV_BIT_MSG_SK> =
        BfvDecCommitVerify::new(expected_commitments, decrypted_shares);

    circuit.verify()
}
