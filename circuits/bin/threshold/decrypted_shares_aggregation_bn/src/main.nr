// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::default::{MAX_MSG_NON_ZERO_COEFFS, T};
use lib::configs::default::threshold::{
    DECRYPTED_SHARES_AGGREGATION_BIT_NOISE, DECRYPTED_SHARES_AGGREGATION_CONFIGS, L,
};
use lib::core::threshold::decrypted_shares_aggregation::DecryptedSharesAggregationBigNum;
use lib::math::polynomial::Polynomial;

fn main(
    decryption_shares: pub [[Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L]; T + 1],
    party_ids: pub [Field; T + 1],
    message: pub Polynomial<MAX_MSG_NON_ZERO_COEFFS>,
    u_global: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,
    crt_quotients: [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L],
) {
    let decrypted_shares_aggregation: DecryptedSharesAggregationBigNum<MAX_MSG_NON_ZERO_COEFFS, L, T, DECRYPTED_SHARES_AGGREGATION_BIT_NOISE> = DecryptedSharesAggregationBigNum::new(
        DECRYPTED_SHARES_AGGREGATION_CONFIGS,
        decryption_shares,
        party_ids,
        message,
        u_global,
        crt_quotients,
    );

    decrypted_shares_aggregation.execute();
}
