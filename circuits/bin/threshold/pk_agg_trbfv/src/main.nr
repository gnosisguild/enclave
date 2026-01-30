// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::default::H;
use lib::configs::default::threshold::{L, N, PK_AGG_TRBFV_BIT_PK, PK_AGG_TRBFV_CONFIGS};
use lib::core::trbfv_pk_agg::TrbfvPublicKeyAggregation;
use lib::math::polynomial::Polynomial;

fn main(
    expected_pk_trbfv_commitments: pub [Field; H],
    pk0: [[Polynomial<N>; L]; H],
    pk1: [[Polynomial<N>; L]; H],
    pk0_agg: [Polynomial<N>; L],
    pk1_agg: [Polynomial<N>; L],
) -> pub Field {
    let pk_agg_trbfv: TrbfvPublicKeyAggregation<N, H, L, PK_AGG_TRBFV_BIT_PK> = TrbfvPublicKeyAggregation::new(
        PK_AGG_TRBFV_CONFIGS,
        expected_pk_trbfv_commitments,
        pk0,
        pk1,
        pk0_agg,
        pk1_agg,
    );

    pk_agg_trbfv.verify()
}
