// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::default::dkg::{
    DKG_SHARE_ENCRYPTION_BIT_CT, DKG_SHARE_ENCRYPTION_BIT_E0, DKG_SHARE_ENCRYPTION_BIT_E1,
    DKG_SHARE_ENCRYPTION_BIT_MSG, DKG_SHARE_ENCRYPTION_BIT_P1, DKG_SHARE_ENCRYPTION_BIT_P2,
    DKG_SHARE_ENCRYPTION_BIT_PK, DKG_SHARE_ENCRYPTION_BIT_R1, DKG_SHARE_ENCRYPTION_BIT_R2,
    DKG_SHARE_ENCRYPTION_BIT_U, DKG_SHARE_ENCRYPTION_CONFIGS, L, N,
};
use lib::core::dkg::share_encryption::ShareEncryption;
use lib::math::polynomial::Polynomial;

fn main(
    expected_pk_commitment: pub Field,
    expected_message_commitment: pub Field,
    pk0is: [Polynomial<N>; L],
    pk1is: [Polynomial<N>; L],
    ct0is: pub [Polynomial<N>; L],
    ct1is: pub [Polynomial<N>; L],
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
) {
    let share_encryption: ShareEncryption<N, L, DKG_SHARE_ENCRYPTION_BIT_PK, DKG_SHARE_ENCRYPTION_BIT_CT, DKG_SHARE_ENCRYPTION_BIT_U, DKG_SHARE_ENCRYPTION_BIT_E0, DKG_SHARE_ENCRYPTION_BIT_E1, DKG_SHARE_ENCRYPTION_BIT_MSG, DKG_SHARE_ENCRYPTION_BIT_R1, DKG_SHARE_ENCRYPTION_BIT_R2, DKG_SHARE_ENCRYPTION_BIT_P1, DKG_SHARE_ENCRYPTION_BIT_P2> = ShareEncryption::new(
        DKG_SHARE_ENCRYPTION_CONFIGS,
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
    share_encryption.execute();
}
