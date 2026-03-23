// SPDX-License-Identifier: LGPL-3.0-only
//
// Standalone sealed-bid auction demo.
//
// Runs a 3-bidder tournament using a single key — no server, no network.
// This exercises the full encrypt → compare → decrypt flow from lib.rs.

use auction_example::*;
use fhe::bfv::SecretKey;
use rand::rngs::OsRng;

fn main() {
    println!("=== Sealed-Bid Auction (BFV SIMD) ===\n");

    let params = build_params();
    println!(
        "Parameters: N={}, t={}, L={} moduli",
        params.degree(),
        params.plaintext(),
        params.moduli().len()
    );
    println!(
        "Bid range: 0-{} ({} bits)\n",
        (1u64 << BID_BITS) - 1,
        BID_BITS
    );

    let sk = SecretKey::random(&params, &mut OsRng);
    let eval_key = build_eval_key(&sk);
    let relin_key = build_relin_key(&sk);

    // Three bidders
    let bidders = vec![("Alice", 750u64), ("Bob", 500), ("Charlie", 900)];

    let encrypted_bids: Vec<_> = bidders
        .iter()
        .map(|(name, value)| {
            let pt = encode_bid(*value, &params);
            let ct = encrypt_bid_sk(&pt, &sk);
            println!("{} encrypted bid: {}", name, value);
            ct
        })
        .collect();

    // Pairwise comparison demo
    println!("\n--- Pairwise Comparisons ---");
    let r1 = compare_greater_than(
        &encrypted_bids[0],
        &encrypted_bids[1],
        &eval_key,
        &relin_key,
        &params,
    );
    println!(
        "Alice(750) > Bob(500)?  {}",
        if decrypt_slot0(&r1, &sk, &params) == 1 {
            "Yes"
        } else {
            "No"
        }
    );

    let r2 = compare_greater_than(
        &encrypted_bids[1],
        &encrypted_bids[2],
        &eval_key,
        &relin_key,
        &params,
    );
    println!(
        "Bob(500) > Charlie(900)? {}",
        if decrypt_slot0(&r2, &sk, &params) == 1 {
            "Yes"
        } else {
            "No"
        }
    );

    // Tournament
    println!("\n--- Tournament ---");
    let (winner_idx, winning_bid) =
        find_winner(&encrypted_bids, &eval_key, &relin_key, &sk, &params);
    println!(
        "Winner: {} with bid {}",
        bidders[winner_idx].0, winning_bid
    );
}
