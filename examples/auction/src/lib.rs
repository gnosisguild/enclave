// SPDX-License-Identifier: LGPL-3.0-only
//
// Sealed-bid auction FHE comparison library.
//
// Provides BFV SIMD-encoded bid encryption and a homomorphic prefix-scan
// comparison circuit.  Used by the standalone demo (`main.rs`), the
// auction server, and the WASM client-side encryption module.

use fhe::bfv::{
    BfvParameters, BfvParametersBuilder, Ciphertext, Encoding, EvaluationKey, EvaluationKeyBuilder,
    Plaintext, PublicKey, RelinearizationKey, SecretKey,
};
use fhe_traits::{FheDecoder, FheDecrypter, FheEncoder, FheEncrypter, Serialize};
use rand::rngs::OsRng;
use std::sync::Arc;

/// Number of bits used to represent each bid.
pub const BID_BITS: usize = 10;

/// Build SIMD-friendly BFV parameters.
///
/// N=2048, t=12289 (prime, 12289 mod 2048 == 1), 6×62-bit moduli.
pub fn build_params() -> Arc<BfvParameters> {
    BfvParametersBuilder::new()
        .set_degree(2048)
        .set_plaintext_modulus(12289)
        .set_moduli_sizes(&[62, 62, 62, 62, 62, 62])
        .build_arc()
        .expect("failed to build BFV parameters")
}

/// Build an evaluation key with column rotations needed for the prefix scan.
pub fn build_eval_key(sk: &SecretKey) -> EvaluationKey {
    let mut builder = EvaluationKeyBuilder::new(sk).expect("failed to create eval key builder");
    let mut shift = 1;
    while shift < BID_BITS {
        builder
            .enable_column_rotation(shift)
            .expect("failed to enable column rotation");
        shift *= 2;
    }
    builder
        .build(&mut OsRng)
        .expect("failed to build evaluation key")
}

/// Build a relinearization key.
pub fn build_relin_key(sk: &SecretKey) -> RelinearizationKey {
    RelinearizationKey::new(sk, &mut OsRng).expect("failed to build relinearization key")
}

/// Multiply two ciphertexts and relinearize.
pub fn mul_relin(a: &Ciphertext, b: &Ciphertext, rk: &RelinearizationKey) -> Ciphertext {
    let mut result = a * b;
    rk.relinearizes(&mut result)
        .expect("relinearization failed");
    result
}

/// Encode a bid value into a SIMD plaintext (binary across slots, MSB first).
pub fn encode_bid(value: u64, params: &Arc<BfvParameters>) -> Plaintext {
    assert!(
        value < (1 << BID_BITS),
        "bid {} exceeds {}-bit range",
        value,
        BID_BITS
    );
    let degree = params.degree();
    let mut slots = vec![0u64; degree];
    for i in 0..BID_BITS {
        slots[i] = (value >> (BID_BITS - 1 - i)) & 1;
    }
    Plaintext::try_encode(&slots, Encoding::simd(), params).expect("failed to encode bid")
}

/// Encrypt a plaintext bid with a secret key (for standalone demo).
pub fn encrypt_bid_sk(pt: &Plaintext, sk: &SecretKey) -> Ciphertext {
    sk.try_encrypt(pt, &mut OsRng)
        .expect("failed to encrypt bid")
}

/// Encrypt a plaintext bid with a public key (for client-side encryption).
pub fn encrypt_bid_pk(pt: &Plaintext, pk: &PublicKey) -> Ciphertext {
    pk.try_encrypt(pt, &mut OsRng)
        .expect("failed to encrypt bid")
}

/// Homomorphic comparison: returns encrypted result where slot 0 = 1 if a > b.
///
/// Uses a parallel prefix scan over SIMD-encoded binary slots.
pub fn compare_greater_than(
    ct_a: &Ciphertext,
    ct_b: &Ciphertext,
    eval_key: &EvaluationKey,
    rk: &RelinearizationKey,
    params: &Arc<BfvParameters>,
) -> Ciphertext {
    let degree = params.degree();
    let ones = Plaintext::try_encode(&vec![1u64; degree], Encoding::simd(), params)
        .expect("failed to encode ones");

    // Per-bit signals
    let diff = ct_a - ct_b;
    let diff_sq = mul_relin(&diff, &diff, rk);
    let mut eq = &(-&diff_sq) + &ones; // 1 when equal
    let ab = mul_relin(ct_a, ct_b, rk);
    let mut gt = ct_a - &ab; // 1 when a_i=1, b_i=0

    // Prefix scan
    let mut shift = 1;
    while shift < BID_BITS {
        let gt_shifted = eval_key
            .rotates_columns_by(&gt, shift)
            .expect("column rotation failed for gt");
        let eq_shifted = eval_key
            .rotates_columns_by(&eq, shift)
            .expect("column rotation failed for eq");
        let eq_times_gt_shifted = mul_relin(&eq, &gt_shifted, rk);
        gt = &gt + &eq_times_gt_shifted;
        eq = mul_relin(&eq, &eq_shifted, rk);
        shift *= 2;
    }

    gt
}

/// Decrypt and read slot 0 from a SIMD ciphertext.
pub fn decrypt_slot0(ct: &Ciphertext, sk: &SecretKey, params: &Arc<BfvParameters>) -> u64 {
    let pt = sk.try_decrypt(ct).expect("decryption failed");
    let slots = Vec::<u64>::try_decode(&pt, Encoding::simd()).expect("decode failed");
    let val = slots[0];
    if val > params.plaintext() / 2 {
        0
    } else {
        val
    }
}

/// Decrypt a bid ciphertext and reconstruct the integer value from SIMD binary slots.
pub fn decrypt_bid(ct: &Ciphertext, sk: &SecretKey, _params: &Arc<BfvParameters>) -> u64 {
    let pt = sk.try_decrypt(ct).expect("decryption failed");
    let slots = Vec::<u64>::try_decode(&pt, Encoding::simd()).expect("decode failed");
    let mut value = 0u64;
    for i in 0..BID_BITS {
        value |= (slots[i] & 1) << (BID_BITS - 1 - i);
    }
    value
}

/// Run a pairwise tournament over encrypted bids and return (winner_index, winning_bid_value).
pub fn find_winner(
    bids: &[Ciphertext],
    eval_key: &EvaluationKey,
    rk: &RelinearizationKey,
    sk: &SecretKey,
    params: &Arc<BfvParameters>,
) -> (usize, u64) {
    assert!(!bids.is_empty(), "no bids to compare");

    let mut winner_idx = 0;
    for i in 1..bids.len() {
        let result = compare_greater_than(&bids[i], &bids[winner_idx], eval_key, rk, params);
        if decrypt_slot0(&result, sk, params) == 1 {
            winner_idx = i;
        }
    }

    let winning_value = decrypt_bid(&bids[winner_idx], sk, params);
    (winner_idx, winning_value)
}

/// Serialize a public key to bytes.
pub fn pk_to_bytes(pk: &PublicKey) -> Vec<u8> {
    pk.to_bytes()
}

/// Serialize a ciphertext to bytes.
pub fn ct_to_bytes(ct: &Ciphertext) -> Vec<u8> {
    ct.to_bytes()
}
