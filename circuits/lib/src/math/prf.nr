// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Poseidon2 PRF used by synchronized decryption.
//!
//! The absorb order matches `fhe.rs` `trbfv::prf`: party ids are 1-based, and
//! ciphertext coefficients are absorbed low degree first as residues in `[0, q)`.
//! Circuit polynomials store the highest degree first and use centered residues.

use crate::math::commitments::compute_packed_prf_key_commitment;
use crate::math::modulo::U64::ModU64;
use crate::math::polynomial::Polynomial;
use crate::math::safe::SafeSponge;

global ABSORB_FLAG: u32 = 0x80000000;

/// Bind a packed native message to its key bits and return the unpacked residue row.
pub fn unpack_key_row<let N: u32, let BITS: u32>(
    packed: [Field; N],
    bits: [bool; BITS],
    q: Field,
    row: u32,
) -> [Field; N] {
    assert(BITS <= N);
    let mut residues = packed;
    for j in 0..N {
        let bit = if (row == 0) & (j < BITS) {
            bits[j] as Field
        } else {
            0
        };
        let value = packed[j] - bit * q;
        value.assert_max_bit_size::<64>();
        (q - 1 - value).assert_max_bit_size::<64>();
        residues[j] = value;
    }
    residues
}

pub fn packed_key<let BITS: u32>(bits: [bool; BITS]) -> Field {
    let mut key: Field = 0;
    let mut place: Field = 1;
    for i in 0..BITS {
        key += (bits[i] as Field) * place;
        place += place;
    }
    key
}

#[test]
fn packed_share_bits_round_trip() {
    assert(unpack_key_row([0, 98, 96, 17], [false, true], 97, 0) == [0, 1, 96, 17]);
    assert(packed_key([true, false, true]) == 5);
}

#[test(should_fail)]
fn packed_share_rejects_a_different_key_bit() {
    let _ = unpack_key_row([0, 98, 96, 17], [false, false], 97, 0);
}

#[test(should_fail)]
fn other_limbs_cannot_carry_key_bits() {
    let _ = unpack_key_row([0, 98, 96, 17], [false, true], 97, 1);
}

/// Domain separator `fhe.rs/trbfv/prf/poseidon2/ctx`.
pub global DS_PRF_CTX: [u8; 64] = [
    0x66, 0x68, 0x65, 0x2e, 0x72, 0x73, 0x2f, 0x74, 0x72, 0x62, 0x66, 0x76, 0x2f, 0x70, 0x72, 0x66,
    0x2f, 0x70, 0x6f, 0x73, 0x65, 0x69, 0x64, 0x6f, 0x6e, 0x32, 0x2f, 0x63, 0x74, 0x78, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Domain separator `fhe.rs/trbfv/prf/poseidon2/eval`.
pub global DS_PRF_EVAL: [u8; 64] = [
    0x66, 0x68, 0x65, 0x2e, 0x72, 0x73, 0x2f, 0x74, 0x72, 0x62, 0x66, 0x76, 0x2f, 0x70, 0x72, 0x66,
    0x2f, 0x70, 0x6f, 0x73, 0x65, 0x69, 0x64, 0x6f, 0x6e, 0x32, 0x2f, 0x65, 0x76, 0x61, 0x6c, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Map a centered residue in `(-q/2, q/2]` to the native residue in `[0, q)`.
pub fn native_residue(centered: Field, q: Field) -> Field {
    let half = (q - 1) / 2;
    let shifted = centered + half;
    let negative = (shifted as u128) < (half as u128);
    centered + (negative as Field) * q
}

/// Unkeyed digest `H(S, ct)`. `decryptors` is a strictly increasing list of 1-based party ids.
pub fn prf_context_digest<let N: u32, let L: u32, let H: u32>(
    decryptors: [Field; H],
    ct0: [Polynomial<N>; L],
    ct1: [Polynomial<N>; L],
    qis: [Field; L],
) -> [Field; 2] {
    let absorb_len: u32 = H + 7 + (2 * L * N);
    // A growing vector makes the compiler repeatedly handle larger intermediate arrays.
    // Keep the released order: |S|, S, level, component count, then (L, N, coefficients).
    let mut input = [0; H + 7 + 2 * L * N];
    input[0] = H as Field;
    for i in 0..H {
        input[1 + i] = decryptors[i];
    }
    input[H + 1] = 0;
    input[H + 2] = 2;
    let ct0_offset = H + 3;
    let ct1_offset = ct0_offset + 2 + L * N;
    input[ct0_offset] = L as Field;
    input[ct0_offset + 1] = N as Field;
    input[ct1_offset] = L as Field;
    input[ct1_offset + 1] = N as Field;
    for row in 0..L {
        for col in 0..N {
            input[ct0_offset + 2 + row * N + col] =
                native_residue(ct0[row].coefficients[N - 1 - col], qis[row]);
            input[ct1_offset + 2 + row * N + col] =
                native_residue(ct1[row].coefficients[N - 1 - col], qis[row]);
        }
    }

    let mut sponge = SafeSponge::start([ABSORB_FLAG | absorb_len, 2], DS_PRF_CTX);
    sponge.absorb(input.as_vector());
    let squeezed = sponge.squeeze();
    sponge.finish();
    [squeezed[0], squeezed[1]]
}

/// Evaluate `F_k` and reduce each coefficient into its CRT residue.
pub fn prf_evaluate<let N: u32, let L: u32>(
    key: Field,
    digest: [Field; 2],
    qis: [Field; L],
) -> [Polynomial<N>; L] {
    let squeeze_len: u32 = L * N;
    let mut sponge = SafeSponge::start([ABSORB_FLAG | 3, squeeze_len], DS_PRF_EVAL);
    sponge.absorb([key, digest[0], digest[1]].as_vector());
    let squeezed = sponge.squeeze_array::<L * N>();
    sponge.finish();

    let mut polys: [Polynomial<N>; L] = [Polynomial::new([0; N]); L];
    for row in 0..L {
        let qmod = ModU64::new(qis[row] as u64);
        let mut values: [Field; N] = [0; N];
        for col in 0..N {
            values[col] = squeezed[(row * N) + col];
        }
        let reduced = qmod.reduce_fields(values);
        let mut coeffs: [Field; N] = [0; N];
        for col in 0..N {
            coeffs[N - 1 - col] = reduced[col];
        }
        polys[row] = Polynomial::new(coeffs);
    }
    polys
}

fn assert_canonical_decryptors<let H: u32, let N_PARTIES: u32>(ids: [Field; H]) {
    for i in 0..H {
        let id = ids[i] as u32;
        assert(ids[i] == id as Field, "Decryptor id is not canonical");
        assert(id >= 1, "Decryptor id is zero");
        assert(id <= N_PARTIES, "Decryptor id is out of range");
        if i > 0 {
            assert((ids[i - 1] as u32) < id, "Decryptor ids are not sorted");
        }
    }
}

fn decryptor_position<let H: u32>(decryptors: [Field; H], party_id_one_based: Field) -> u32 {
    let mut seen = false;
    let mut position = 0;
    for i in 0..H {
        if decryptors[i] == party_id_one_based {
            seen = true;
            position = i;
        }
    }
    assert(seen, "Party is outside the decryptor set");
    position
}

fn add_mask_difference<let N: u32, let L: u32>(
    mask: [Polynomial<N>; L],
    positive: [Polynomial<N>; L],
    negative: [Polynomial<N>; L],
    qis: [Field; L],
) -> [Polynomial<N>; L] {
    let mut next: [Polynomial<N>; L] = [Polynomial::new([0; N]); L];
    for row in 0..L {
        let q = qis[row];
        let qmod = ModU64::new(q as u64);
        let mut coeffs: [Field; N] = [0; N];
        for col in 0..N {
            let combined = mask[row].coefficients[col] + positive[row].coefficients[col] + q
                - negative[row].coefficients[col];
            coeffs[col] = qmod.reduce_mod(combined);
        }
        next[row] = Polynomial::new(coeffs);
    }
    next
}

/// Build `r_i = Σ_{j in S, j ≠ i} (F_{k_ij}(H) - F_{k_ji}(H))`.
///
/// Outgoing key `k_ij` is committed under recipient `j`. Incoming key `k_ji` is
/// committed under recipient `i`.
pub fn prf_decryption_mask<let N: u32, let L: u32, let N_PARTIES: u32, let H: u32>(
    party_idx: u32,
    decryptors: [Field; H],
    outgoing_keys: [Field; N_PARTIES],
    incoming_keys: [Field; N_PARTIES],
    outgoing_commitments: [Field; N_PARTIES],
    incoming_commitments: [Field; N_PARTIES],
    ct0: [Polynomial<N>; L],
    ct1: [Polynomial<N>; L],
    qis: [Field; L],
) -> [Polynomial<N>; L] {
    // H here is the designated set length (T+1), not DKG's selected-dealer count.
    assert(party_idx < N_PARTIES, "Party index is out of range");
    assert(H > 0);
    assert_canonical_decryptors::<H, N_PARTIES>(decryptors);
    let self_id = (party_idx + 1) as Field;
    let own_position = decryptor_position(decryptors, self_id);

    let digest = prf_context_digest(decryptors, ct0, ct1, qis);
    let mut mask: [Polynomial<N>; L] = [Polynomial::new([0; N]); L];

    // A witness-dependent `if` does not skip circuit work. Select exactly H-1
    // peers first, so only their two PRF streams are expanded by the compiler.
    for slot in 0..(H - 1) {
        let peer_position = if slot < own_position { slot } else { slot + 1 };
        let other = (decryptors[peer_position] as u32) - 1;
        assert(
            compute_packed_prf_key_commitment(other as Field, outgoing_keys[other])
                == outgoing_commitments[other],
            "Outgoing PRF key commitment mismatch",
        );
        assert(
            compute_packed_prf_key_commitment(party_idx as Field, incoming_keys[other])
                == incoming_commitments[other],
            "Incoming PRF key commitment mismatch",
        );
        let outgoing = prf_evaluate(outgoing_keys[other], digest, qis);
        let incoming = prf_evaluate(incoming_keys[other], digest, qis);
        mask = add_mask_difference(mask, outgoing, incoming, qis);
    }

    mask
}

fn mask_fixture(owner: u32, ids: [Field; 3]) -> [Field; 8] {
    let mut outgoing = [0; 5];
    let mut incoming = [0; 5];
    let mut outgoing_commits = [0; 5];
    let mut incoming_commits = [0; 5];
    for peer in 0..5 {
        outgoing[peer] = (10 * owner + peer + 1) as Field;
        incoming[peer] = (10 * (peer + 1) + owner) as Field;
        outgoing_commits[peer] = compute_packed_prf_key_commitment(peer as Field, outgoing[peer]);
        incoming_commits[peer] =
            compute_packed_prf_key_commitment((owner - 1) as Field, incoming[peer]);
    }
    let ct0 = [Polynomial::new([7, 6, 5, 4, 3, 2, 1, 0])];
    let ct1 = [Polynomial::new([15, 14, 13, 12, 11, 10, 9, 8])];
    prf_decryption_mask(
        owner - 1,
        ids,
        outgoing,
        incoming,
        outgoing_commits,
        incoming_commits,
        ct0,
        ct1,
        [65537],
    )[0]
        .coefficients
}

#[test]
fn non_prefix_masks_match_pre_optimization_vectors_and_cancel() {
    let ids: [Field; 3] = [1, 3, 5];
    // Frozen outputs of the previously verified implementation, descending coefficients.
    let expected: [[Field; 8]; 3] = [
        [0x4dcc, 0xb0a8, 0x56b4, 0x0a14, 0xbe5f, 0xb32d, 0xb3b9, 0xf87b],
        [0xe4b4, 0x1f8c, 0x35bc, 0x44b7, 0xbd2d, 0x8940, 0x251e, 0x882e],
        [0xcd82, 0x2fcd, 0x7391, 0xb136, 0x8476, 0xc395, 0x272a, 0x7f59],
    ];
    for i in 0..3 {
        assert(mask_fixture(ids[i] as u32, ids) == expected[i]);
    }
    for coefficient in 0..8 {
        let sum = expected[0][coefficient] + expected[1][coefficient] + expected[2][coefficient];
        assert((sum == 0) | (sum == 65537) | (sum == 2 * 65537));
    }
}

#[test(should_fail_with = "Party is outside the decryptor set")]
fn absent_owner_cannot_supply_a_peer_position() {
    let _ = mask_fixture(2, [1, 3, 5]);
}

#[test(should_fail_with = "Decryptor ids are not sorted")]
fn peer_selection_rejects_unsorted_ids() {
    let _ = mask_fixture(3, [1, 5, 3]);
}

#[test(should_fail_with = "Decryptor ids are not sorted")]
fn peer_selection_rejects_duplicate_ids() {
    let _ = mask_fixture(3, [1, 3, 3]);
}

#[test(should_fail_with = "Decryptor id is zero")]
fn peer_selection_rejects_zero_ids() {
    let _ = mask_fixture(3, [0, 3, 5]);
}

#[test(should_fail_with = "Decryptor id is out of range")]
fn peer_selection_rejects_out_of_range_ids() {
    let _ = mask_fixture(3, [1, 3, 6]);
}

#[test]
fn native_residue_preserves_centered_signs() {
    assert(native_residue(0, 97) == 0);
    assert(native_residue(48, 97) == 48);
    assert(native_residue(-1, 97) == 96);
    assert(native_residue(-48, 97) == 49);
}

#[test]
fn context_digest_matches_multi_crt_legacy_vector() {
    let qis: [Field; 3] = [576460752317792257, 576460752317399041, 576460752316776449];
    let mut ct0 = [Polynomial::new([0; 8]); 3];
    let mut ct1 = [Polynomial::new([0; 8]); 3];
    for row in 0..3 {
        let half = (qis[row] - 1) / 2;
        ct0[row] = Polynomial::new([-half, half, -1, 0, 1, -7, 11, 23]);
        ct1[row] = Polynomial::new([23, 11, -7, 1, 0, -1, half, -half]);
    }
    // Frozen from the pre-optimization implementation with distinct CRT limbs and signed edges.
    assert(
        prf_context_digest([1, 3, 5], ct0, ct1, qis)
            == [
                0x1288e74dc60c90b73a07bf868145286b4caccf4a07d5e19086dbae626996a835,
                0x25a06f5c1009f61e80deac85d497b058e1cc6ba6e1782fb1d3cff8d413baac67,
            ],
    );
}
