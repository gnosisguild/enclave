// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::insecure::trbfv::{DEC_SHARES_AGG_BIT_NOISE, DEC_SHARES_AGG_CONFIGS, L};
use lib::core::trbfv_dec_shares_agg::DecryptionSharesAggregation;
use lib::math::polynomial::Polynomial;

/// Max number of non-zero coefficients in the message polynomial.
pub global MAX_MSG_NON_ZERO_COEFFS: u32 = 80;
/// Threshold.
pub global T: u32 = 2;

fn main(
    decryption_shares: pub [[Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L]; T + 1],
    party_ids: pub [Field; T + 1],
    message: pub Polynomial<MAX_MSG_NON_ZERO_COEFFS>,
    u_global: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,
    crt_quotients: [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L],
) {
    let dec_share_agg: DecryptionSharesAggregation<MAX_MSG_NON_ZERO_COEFFS, L, T, DEC_SHARES_AGG_BIT_NOISE> = DecryptionSharesAggregation::new(
        DEC_SHARES_AGG_CONFIGS,
        decryption_shares,
        party_ids,
        message,
        u_global,
        crt_quotients,
    );

    dec_share_agg.verify_no_bn();
}
