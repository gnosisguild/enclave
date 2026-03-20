// SPDX-License-Identifier: LGPL-3.0-only
//
// Sealed-bid auction demo using BFV SIMD encoding.
//
// Two bidders encode their bids in binary across SIMD slots of a single
// ciphertext each. A homomorphic comparison circuit determines the winner
// without decrypting individual bids.
//
// This is a standalone mock — no ciphernodes, no threshold key. Just a
// single secret key for the full encrypt → compare → decrypt flow.

use fhe::bfv::{
    BfvParameters, BfvParametersBuilder, Ciphertext, EvaluationKey, EvaluationKeyBuilder,
    Encoding, Plaintext, SecretKey,
};
use fhe_traits::{FheDecoder, FheDecrypter, FheEncoder, FheEncrypter};
use rand::rngs::OsRng;
use std::sync::Arc;

/// Number of bits used to represent each bid.
const BID_BITS: usize = 10;

/// Build SIMD-friendly BFV parameters.
///
/// For SIMD encoding, the plaintext modulus t must satisfy t ≡ 1 (mod N).
/// We use N=512 and t=12289 for testing (t = 24*512 + 1, prime).
fn build_params() -> Arc<BfvParameters> {
    let degree = 512;
    let plaintext_modulus = 12289; // prime, 12289 mod 512 == 1
    // Two ~36-bit NTT-friendly primes (from the existing prime pool)
    let moduli = &[0xffffee001, 0xffffc4001];

    BfvParametersBuilder::new()
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(moduli)
        .build_arc()
        .expect("failed to build BFV parameters")
}

/// Build an evaluation key with column rotations needed for the prefix scan.
///
/// We need rotations by powers of 2: 1, 2, 4, ..., up to BID_BITS/2.
fn build_eval_key(sk: &SecretKey) -> EvaluationKey {
    let mut builder = EvaluationKeyBuilder::new(sk).expect("failed to create eval key builder");

    // Enable column rotations for each power-of-2 shift
    let mut shift = 1;
    while shift < BID_BITS {
        builder
            .enable_column_rotation(shift)
            .expect("failed to enable column rotation");
        shift *= 2;
    }

    builder.build(&mut OsRng).expect("failed to build evaluation key")
}

/// Encode a bid value into a SIMD plaintext.
///
/// The bid is binary-encoded across the first BID_BITS slots:
///   slot 0 = MSB, slot 1 = next bit, ..., slot (BID_BITS-1) = LSB
///
/// Remaining slots are zero.
fn encode_bid(value: u64, params: &Arc<BfvParameters>) -> Plaintext {
    assert!(
        value < (1 << BID_BITS),
        "bid {} exceeds {}-bit range",
        value,
        BID_BITS
    );

    let degree = params.degree();
    let mut slots = vec![0u64; degree];

    for i in 0..BID_BITS {
        // MSB first: slot 0 = bit (BID_BITS-1), slot (BID_BITS-1) = bit 0
        slots[i] = (value >> (BID_BITS - 1 - i)) & 1;
    }

    Plaintext::try_encode(&slots, Encoding::simd(), params).expect("failed to encode bid")
}

/// Encrypt a plaintext bid.
fn encrypt_bid(pt: &Plaintext, sk: &SecretKey) -> Ciphertext {
    sk.try_encrypt(pt, &mut OsRng).expect("failed to encrypt bid")
}

/// Homomorphic comparison: returns an encrypted result where slot 0 = 1 if a > b, 0 otherwise.
///
/// Algorithm (parallel prefix scan on SIMD slots):
///
/// 1. Compute per-bit signals:
///    - diff = a - b (slot-wise subtraction)
///    - eq_i  = 1 if a_i == b_i  → computed as 1 - diff_i^2
///    - gt_i  = 1 if a_i > b_i   → computed as a_i * (1 - b_i) = a_i - a_i*b_i
///
/// 2. Prefix scan (tree reduction):
///    For each round j = 0, 1, ..., ceil(log2(BID_BITS))-1:
///      shift = 2^j
///      gt = gt + rotate(eq, shift) * rotate(gt, -shift)  -- actually we merge pairs
///
///    The merge rule for (gt, eq) pairs:
///      (gt_high, eq_high) ∘ (gt_low, eq_low) = (gt_high + eq_high * gt_low, eq_high * eq_low)
///
///    After log2(BID_BITS) rounds, slot 0 holds: 1 if A > B, 0 if A <= B.
fn compare_greater_than(
    ct_a: &Ciphertext,
    ct_b: &Ciphertext,
    eval_key: &EvaluationKey,
    params: &Arc<BfvParameters>,
) -> Ciphertext {
    let degree = params.degree();

    // Encode constants
    let ones = Plaintext::try_encode(&vec![1u64; degree], Encoding::simd(), params)
        .expect("failed to encode ones");

    // Step 1: Compute per-bit eq and gt
    //   diff = a - b
    let diff = ct_a - ct_b;

    //   diff_sq = diff * diff (slot-wise, depth 1)
    let diff_sq = &diff * &diff;

    //   eq = 1 - diff^2: equals 1 when a_i == b_i, 0 when different
    let mut eq = &(-&diff_sq) + &ones;

    //   gt = a * (1 - b) = a - a*b: equals 1 when a_i=1,b_i=0
    let ab = ct_a * ct_b;
    let mut gt = ct_a - &ab;

    // Step 2: Prefix scan using column rotations
    //
    // We think of the slots as ordered MSB to LSB (slot 0 = MSB).
    // After the scan, slot 0 will hold the comparison result.
    //
    // At each round, we merge pairs that are `shift` apart:
    //   gt_merged[i] = gt[i] + eq[i] * gt[i + shift]
    //   eq_merged[i] = eq[i] * eq[i + shift]
    //
    // This doubles the "reach" of each slot's comparison each round.
    let mut shift = 1;
    while shift < BID_BITS {
        // Rotate gt and eq to bring slot[i+shift] into slot[i]
        let gt_shifted = eval_key
            .rotates_columns_by(&gt, shift)
            .expect("column rotation failed for gt");
        let eq_shifted = eval_key
            .rotates_columns_by(&eq, shift)
            .expect("column rotation failed for eq");

        // Merge: gt = gt + eq * gt_shifted
        let eq_times_gt_shifted = &eq * &gt_shifted;
        gt = &gt + &eq_times_gt_shifted;

        // Merge: eq = eq * eq_shifted
        eq = &eq * &eq_shifted;

        shift *= 2;
    }

    // gt now has the comparison result in slot 0
    gt.clone()
}

/// Decrypt and read slot 0 from a SIMD ciphertext.
fn decrypt_slot0(ct: &Ciphertext, sk: &SecretKey, params: &Arc<BfvParameters>) -> u64 {
    let pt = sk.try_decrypt(ct).expect("decryption failed");
    let slots = Vec::<u64>::try_decode(&pt, Encoding::simd()).expect("decode failed");
    // The result might wrap around the plaintext modulus for negative values
    let val = slots[0];
    if val > params.plaintext() / 2 {
        0 // Negative means a <= b
    } else {
        val
    }
}

fn main() {
    println!("=== Sealed-Bid Auction (BFV SIMD) ===\n");

    // Setup
    let params = build_params();
    println!(
        "Parameters: N={}, t={}, L={} moduli",
        params.degree(),
        params.plaintext(),
        params.moduli().len()
    );
    println!("Bid range: 0-{} ({} bits)\n", (1u64 << BID_BITS) - 1, BID_BITS);

    let sk = SecretKey::random(&params, &mut OsRng);
    let eval_key = build_eval_key(&sk);

    // Bidder A: bid = 750
    let bid_a = 750;
    let pt_a = encode_bid(bid_a, &params);
    let ct_a = encrypt_bid(&pt_a, &sk);
    println!("Bidder A encrypted bid: {}", bid_a);

    // Bidder B: bid = 500
    let bid_b = 500;
    let pt_b = encode_bid(bid_b, &params);
    let ct_b = encrypt_bid(&pt_b, &sk);
    println!("Bidder B encrypted bid: {}", bid_b);

    // Homomorphic comparison: is A > B?
    println!("\nComputing comparison homomorphically...");
    let ct_result = compare_greater_than(&ct_a, &ct_b, &eval_key, &params);

    // Decrypt only the comparison result
    let a_wins = decrypt_slot0(&ct_result, &sk, &params);
    println!("Result (slot 0): {}", a_wins);

    if a_wins == 1 {
        println!("\nBidder A wins!");
        // In a real auction, only decrypt the winning bid
        let pt_winner = sk.try_decrypt(&ct_a).expect("decryption failed");
        let winner_slots =
            Vec::<u64>::try_decode(&pt_winner, Encoding::simd()).expect("decode failed");
        let mut winning_bid = 0u64;
        for i in 0..BID_BITS {
            winning_bid |= winner_slots[i] << (BID_BITS - 1 - i);
        }
        println!("Winning bid: {}", winning_bid);
    } else {
        println!("\nBidder B wins (or tie)!");
        let pt_winner = sk.try_decrypt(&ct_b).expect("decryption failed");
        let winner_slots =
            Vec::<u64>::try_decode(&pt_winner, Encoding::simd()).expect("decode failed");
        let mut winning_bid = 0u64;
        for i in 0..BID_BITS {
            winning_bid |= winner_slots[i] << (BID_BITS - 1 - i);
        }
        println!("Winning bid: {}", winning_bid);
    }

    // Also test the reverse
    println!("\n--- Reverse comparison (B > A?) ---");
    let ct_result_rev = compare_greater_than(&ct_b, &ct_a, &eval_key, &params);
    let b_wins = decrypt_slot0(&ct_result_rev, &sk, &params);
    println!("B > A result: {}", b_wins);

    // Test equal bids
    println!("\n--- Equal bids test ---");
    let bid_c = 500;
    let pt_c = encode_bid(bid_c, &params);
    let ct_c = encrypt_bid(&pt_c, &sk);
    let ct_equal = compare_greater_than(&ct_c, &ct_b, &eval_key, &params);
    let equal_result = decrypt_slot0(&ct_equal, &sk, &params);
    println!("500 > 500? Result: {} (expected: 0)", equal_result);
}
