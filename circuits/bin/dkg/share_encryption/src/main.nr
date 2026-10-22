// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::default::dkg::{
    L, L_THRESHOLD, N, QIS_THRESHOLD, SHARE_COMPUTATION_PRF_KEY_BITS, SHARE_ENCRYPTION_BIT_CT,
    SHARE_ENCRYPTION_BIT_E0, SHARE_ENCRYPTION_BIT_E1, SHARE_ENCRYPTION_BIT_MSG,
    SHARE_ENCRYPTION_BIT_PK, SHARE_ENCRYPTION_BIT_Q0, SHARE_ENCRYPTION_BIT_Q0_DIFF,
    SHARE_ENCRYPTION_BIT_Q1, SHARE_ENCRYPTION_BIT_U, SHARE_ENCRYPTION_BIT_Z,
    SHARE_ENCRYPTION_CONFIGS, SHARE_ENCRYPTION_CT0_BIT_R, SHARE_ENCRYPTION_CT1_BIT_R,
};
use lib::configs::default::N_PARTIES;
use lib::core::dkg::share_encryption::ShareEncryption;
use lib::math::commitments::compute_packed_prf_key_commitment;
use lib::math::polynomial::Polynomial;
use lib::math::prf::{packed_key, unpack_key_row};

fn main(
    expected_pk_commitment: pub Field,
    expected_message_commitment: pub Field,
    recipient_party_idx: pub u32,
    row_idx: pub u32,
    prf_key_commitment: pub Field,
    pk0is: [Polynomial<N>; L],
    pk1is: [Polynomial<N>; L],
    ct0is: [Polynomial<N>; L],
    ct1is: [Polynomial<N>; L],
    u: Polynomial<N>,
    e0: Polynomial<N>,
    e1: Polynomial<N>,
    message: Polynomial<N>,
    ct0_r: [Polynomial<N>; L],
    ct1_r: [Polynomial<N>; L],
    z: [Polynomial<N>; 1],
    prf_key: [bool; SHARE_COMPUTATION_PRF_KEY_BITS],
) -> pub Field {
    assert(recipient_party_idx < N_PARTIES);
    assert(row_idx < L_THRESHOLD);
    let mut native = [0; N];
    for j in 0..N {
        native[j] = message.coefficients[N - 1 - j];
    }
    let _ = unpack_key_row(native, prf_key, QIS_THRESHOLD[row_idx], row_idx);
    assert(
        compute_packed_prf_key_commitment(recipient_party_idx as Field, packed_key(prf_key))
            == prf_key_commitment,
        "PRF key commitment mismatch",
    );
    let share_encryption: ShareEncryption<N, L, SHARE_ENCRYPTION_BIT_PK, SHARE_ENCRYPTION_BIT_CT, SHARE_ENCRYPTION_BIT_U, SHARE_ENCRYPTION_BIT_E0, SHARE_ENCRYPTION_BIT_E1, SHARE_ENCRYPTION_BIT_MSG, SHARE_ENCRYPTION_CT0_BIT_R, SHARE_ENCRYPTION_CT1_BIT_R, SHARE_ENCRYPTION_BIT_Z, SHARE_ENCRYPTION_BIT_Q0, SHARE_ENCRYPTION_BIT_Q0_DIFF, SHARE_ENCRYPTION_BIT_Q1> = ShareEncryption::new(
        SHARE_ENCRYPTION_CONFIGS,
        expected_pk_commitment,
        expected_message_commitment,
        pk0is,
        pk1is,
        ct0is,
        ct1is,
        u,
        e0,
        e1,
        message,
        ct0_r,
        ct1_r,
        z,
    );
    share_encryption.execute()
}
