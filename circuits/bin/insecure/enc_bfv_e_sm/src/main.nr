// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::insecure::bfv::{
    ENC_BFV_BIT_CT, ENC_BFV_BIT_E0, ENC_BFV_BIT_E1, ENC_BFV_BIT_MSG, ENC_BFV_BIT_P1, ENC_BFV_BIT_P2,
    ENC_BFV_BIT_PK, ENC_BFV_BIT_R1, ENC_BFV_BIT_R2, ENC_BFV_BIT_U, ENC_BFV_CONFIGS_E_SM, L, N,
};
use lib::core::bfv_enc::EncryptionBfv;
use lib::math::polynomial::Polynomial;

fn main(
    expected_pk_commitment: Field,
    expected_message_commitment: Field,
    pk0is: [Polynomial<N>; L],
    pk1is: [Polynomial<N>; L],
    ct0is: [Polynomial<N>; L],
    ct1is: [Polynomial<N>; L],
    u: Polynomial<N>,
    e0: Polynomial<N>,
    e0is: [Polynomial<N>; L],
    e0_quotients: [Polynomial<N>; L],
    e1: Polynomial<N>,
    message: Polynomial<N>,
    r1is: [Polynomial<(2 * N) - 1>; L],
    r2is: [Polynomial<N - 1>; L],
    p1is: [Polynomial<(2 * N) - 1>; L],
    p2is: [Polynomial<N - 1>; L],
) -> pub bool {
    let enc_bfv: EncryptionBfv<N, L, ENC_BFV_BIT_PK, ENC_BFV_BIT_CT, ENC_BFV_BIT_U, ENC_BFV_BIT_E0, ENC_BFV_BIT_E1, ENC_BFV_BIT_MSG, ENC_BFV_BIT_R1, ENC_BFV_BIT_R2, ENC_BFV_BIT_P1, ENC_BFV_BIT_P2> = EncryptionBfv::new(
        ENC_BFV_CONFIGS_E_SM,
        expected_pk_commitment,
        expected_message_commitment,
        pk0is,
        pk1is,
        ct0is,
        ct1is,
        u,
        e0,
        e0is,
        e0_quotients,
        e1,
        message,
        r1is,
        r2is,
        p1is,
        p2is,
    );
    enc_bfv.verify()
}
