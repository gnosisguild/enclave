// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::default::dkg::{L, N};
use lib::configs::default::dkg::PK_BFV_BIT_PK;
use lib::core::bfv_pk::BfvPkCommit;
use lib::math::polynomial::Polynomial;

fn main(pk0is: [Polynomial<N>; L], pk1is: [Polynomial<N>; L]) -> pub Field {
    let pk_bfv: BfvPkCommit<N, L, PK_BFV_BIT_PK> = BfvPkCommit::new(pk0is, pk1is);
    pk_bfv.verify()
}
