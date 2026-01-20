// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::production::trbfv::{
    L, N, PK_TRBFV_BIT_E_SM, PK_TRBFV_BIT_EEK, PK_TRBFV_BIT_PK, PK_TRBFV_BIT_R1, PK_TRBFV_BIT_R2,
    PK_TRBFV_BIT_SK, PK_TRBFV_CONFIGS,
};
use lib::core::trbfv_pk::TrbfvPublicKey;
use lib::math::polynomial::Polynomial;

fn main(
    a: pub [Polynomial<N>; L],
    eek: Polynomial<N>,
    sk: Polynomial<N>,
    e_sm: Polynomial<N>,
    r1is: [Polynomial<(2 * N) - 1>; L],
    r2is: [Polynomial<N - 1>; L],
    pk0is: [Polynomial<N>; L],
    pk1is: [Polynomial<N>; L],
) -> pub (Field, Field, Field) {
    let pk_trbfv: TrbfvPublicKey<N, L, PK_TRBFV_BIT_EEK, PK_TRBFV_BIT_SK, PK_TRBFV_BIT_E_SM, PK_TRBFV_BIT_R1, PK_TRBFV_BIT_R2, PK_TRBFV_BIT_PK> =
        TrbfvPublicKey::new(PK_TRBFV_CONFIGS, a, eek, sk, e_sm, r1is, r2is, pk0is, pk1is);
    pk_trbfv.verify()
}
