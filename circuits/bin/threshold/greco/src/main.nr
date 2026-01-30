// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::default::threshold::{
    GRECO_BIT_CT, GRECO_BIT_E0, GRECO_BIT_E1, GRECO_BIT_K, GRECO_BIT_P1, GRECO_BIT_P2, GRECO_BIT_PK,
    GRECO_BIT_R1, GRECO_BIT_R2, GRECO_BIT_U, GRECO_CONFIGS, L, N,
};
use lib::core::greco::Greco;
use lib::math::polynomial::Polynomial;

fn main(
    pk_commitment: pub Field,
    pk0is: [Polynomial<N>; L],
    pk1is: [Polynomial<N>; L],
    ct0is: [Polynomial<N>; L],
    ct1is: [Polynomial<N>; L],
    u: Polynomial<N>,
    e0: Polynomial<N>,
    e1: Polynomial<N>,
    e0is: [Polynomial<N>; L],
    e0_quotients: [Polynomial<N>; L],
    k1: Polynomial<N>,
    r1is: [Polynomial<(2 * N) - 1>; L],
    r2is: [Polynomial<N - 1>; L],
    p1is: [Polynomial<(2 * N) - 1>; L],
    p2is: [Polynomial<N - 1>; L],
) {
    let greco: Greco<N, L, GRECO_BIT_PK, GRECO_BIT_CT, GRECO_BIT_U, GRECO_BIT_E0, GRECO_BIT_E1, GRECO_BIT_K, GRECO_BIT_R1, GRECO_BIT_R2, GRECO_BIT_P1, GRECO_BIT_P2> = Greco::new(
        GRECO_CONFIGS,
        pk_commitment,
        pk0is,
        pk1is,
        ct0is,
        ct1is,
        u,
        e0,
        e0is,
        e0_quotients,
        e1,
        k1,
        r1is,
        r2is,
        p1is,
        p2is,
    );
    let is_greco_valid = greco.verify();
    assert(is_greco_valid);
}
