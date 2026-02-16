// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crisp_lib::commitments::{compute_ct_commitment, compute_k1_commitment};
use enclave_lib::configs::default::threshold::{
    L, N, USER_DATA_ENCRYPTION_BIT_CT, USER_DATA_ENCRYPTION_BIT_E0, USER_DATA_ENCRYPTION_BIT_E1,
    USER_DATA_ENCRYPTION_BIT_K, USER_DATA_ENCRYPTION_BIT_P1, USER_DATA_ENCRYPTION_BIT_P2,
    USER_DATA_ENCRYPTION_BIT_PK, USER_DATA_ENCRYPTION_BIT_R1, USER_DATA_ENCRYPTION_BIT_R2,
    USER_DATA_ENCRYPTION_BIT_U, USER_DATA_ENCRYPTION_CONFIGS,
};
use enclave_lib::core::threshold::user_data_encryption::UserDataEncryption;
use enclave_lib::math::polynomial::Polynomial;

fn main(
    pk_commitment: pub Field,
    pk0is: [Polynomial<N>; L],
    pk1is: [Polynomial<N>; L],
    ct0is: [Polynomial<N>; L],
    ct1is: [Polynomial<N>; L],
    u: Polynomial<N>,
    e0: Polynomial<N>,
    e0is: [Polynomial<N>; L],
    e0_quotients: [Polynomial<N>; L],
    e1: Polynomial<N>,
    k1: Polynomial<N>,
    r1is: [Polynomial<(2 * N) - 1>; L],
    r2is: [Polynomial<N - 1>; L],
    p1is: [Polynomial<(2 * N) - 1>; L],
    p2is: [Polynomial<N - 1>; L],
) -> pub (Field, Field) {
    let greco: UserDataEncryption<N, L, USER_DATA_ENCRYPTION_BIT_PK, USER_DATA_ENCRYPTION_BIT_CT, USER_DATA_ENCRYPTION_BIT_U, USER_DATA_ENCRYPTION_BIT_E0, USER_DATA_ENCRYPTION_BIT_E1, USER_DATA_ENCRYPTION_BIT_K, USER_DATA_ENCRYPTION_BIT_R1, USER_DATA_ENCRYPTION_BIT_R2, USER_DATA_ENCRYPTION_BIT_P1, USER_DATA_ENCRYPTION_BIT_P2> = UserDataEncryption::new(
        USER_DATA_ENCRYPTION_CONFIGS,
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

    assert(greco.execute());

    let k1_commitment = compute_k1_commitment::<N, USER_DATA_ENCRYPTION_BIT_K>(k1);
    let ct_commitment = compute_ct_commitment::<N, L, USER_DATA_ENCRYPTION_BIT_CT>(ct0is, ct1is);

    (k1_commitment, ct_commitment)
}
