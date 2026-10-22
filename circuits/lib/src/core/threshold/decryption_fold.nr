// SPDX-License-Identifier: LGPL-3.0-only

use crate::configs::default::{N_PARTIES, T};
use crate::configs::recursive::{
    C6_FOLD_PREFIX_LEN, C6_FOLD_PUBLIC_LEN, C6_INNER_PUBLIC_LEN, C6_KEY_SLOTS,
};

/// Fold one designated party without permitting a different domain or designated set.
pub fn fold_statement(
    leaf: [Field; C6_INNER_PUBLIC_LEN],
    prior: [Field; C6_FOLD_PUBLIC_LEN],
    first: bool,
    slot: u32,
    ) -> (Field, Field, [Field; T + 1], [Field; T + 1], [Field; T + 1], [Field; T + 1], [Field; C6_KEY_SLOTS], [Field; C6_KEY_SLOTS], [Field; T + 1]) {
    assert(slot < T + 1);
    assert(leaf[5] + 1 == leaf[6 + slot], "C6 party does not own its fold slot");
    let mut ids = [0; T + 1];
    let id_base = C6_FOLD_PREFIX_LEN + 4 * (T + 1) + 2 * C6_KEY_SLOTS;
    for i in 0..T + 1 {
        ids[i] = leaf[6 + i];
        if !first {
            assert(prior[id_base + i] == ids[i], "Mixed decryptor sets");
        }
    }
    if !first {
        assert(prior[5] == leaf[3]);
        assert(prior[6] == leaf[4]);
    }
    let mut sk = [0; T + 1];
    let mut c0 = [0; T + 1];
    let mut ct = [0; T + 1];
    let mut d = [0; T + 1];
    let mut outgoing = [0; C6_KEY_SLOTS];
    let mut incoming = [0; C6_KEY_SLOTS];
    let key_base = C6_FOLD_PREFIX_LEN + 4 * (T + 1);
    for i in 0..T + 1 {
        if !first {
            sk[i] = prior[C6_FOLD_PREFIX_LEN + i];
            c0[i] = prior[C6_FOLD_PREFIX_LEN + T + 1 + i];
            ct[i] = prior[C6_FOLD_PREFIX_LEN + 2 * (T + 1) + i];
            d[i] = prior[C6_FOLD_PREFIX_LEN + 3 * (T + 1) + i];
        }
        if i == slot {
            assert(sk[i] == 0);
            assert(c0[i] == 0);
            assert(ct[i] == 0);
            assert(d[i] == 0);
            sk[i] = leaf[0];
            c0[i] = leaf[2];
            ct[i] = leaf[1];
            d[i] = leaf[C6_INNER_PUBLIC_LEN - 1];
        }
        for peer in 0..N_PARTIES {
            let k = i * N_PARTIES + peer;
            if !first {
                outgoing[k] = prior[key_base + k];
                incoming[k] = prior[key_base + C6_KEY_SLOTS + k];
            }
            if i == slot {
                assert(outgoing[k] == 0);
                assert(incoming[k] == 0);
                outgoing[k] = leaf[6 + T + 1 + peer];
                incoming[k] = leaf[6 + T + 1 + N_PARTIES + peer];
            }
        }
    }
    (leaf[3], leaf[4], sk, c0, ct, d, outgoing, incoming, ids)
}
