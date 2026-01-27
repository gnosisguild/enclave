// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use num_bigint::BigUint;
use num_traits::One;
use std::collections::BTreeMap;

use crate::search::constants::NTT_PRIMES_BY_BITS;
use crate::search::utils::{log2_big, parse_hex_big};

#[derive(Debug, Clone)]
pub struct PrimeItem {
    pub bitlen: u8,
    pub value: BigUint,
    pub log2: f64,
    pub hex: String,
}

/// Build a flat list of all primes with precomputed log2 and hex strings.
pub fn build_prime_items() -> Vec<PrimeItem> {
    let mut vec = Vec::new();
    for (bits, arr) in NTT_PRIMES_BY_BITS.iter() {
        if *bits == 63 || *bits == 62 || *bits == 61 {
            continue;
        }
        for &phex in arr {
            let v = parse_hex_big(phex);
            vec.push(PrimeItem {
                bitlen: *bits,
                log2: log2_big(&v),
                hex: phex.to_string(),
                value: v,
            });
        }
    }
    vec
}

/// Build prime items for second parameter set (includes 62-bit primes, excludes 61 and 63-bit)
pub fn build_prime_items_for_second() -> Vec<PrimeItem> {
    let mut vec = Vec::new();
    for (bits, arr) in NTT_PRIMES_BY_BITS.iter() {
        if *bits == 63 || *bits == 61 {
            continue;
        }
        for &phex in arr {
            let v = parse_hex_big(phex);
            vec.push(PrimeItem {
                bitlen: *bits,
                log2: log2_big(&v),
                hex: phex.to_string(),
                value: v,
            });
        }
    }
    vec
}

pub fn select_max_q_under_cap(limit_log2: f64, all: &[PrimeItem]) -> Vec<PrimeItem> {
    // Greedy: take largest primes from larger bit-lengths first, mixing buckets,
    // ensuring log2(q) stays under the cap as we add
    let mut by_bits: BTreeMap<u8, Vec<PrimeItem>> = BTreeMap::new();
    for p in all {
        by_bits.entry(p.bitlen).or_default().push(p.clone());
    }
    for v in by_bits.values_mut() {
        v.sort_by(|a, b| b.value.cmp(&a.value));
    }

    let mut sel: Vec<PrimeItem> = Vec::new();
    let mut q = BigUint::one();
    let mut qlog = 0.0f64;

    for bb in (40u8..=60u8).rev() {
        if let Some(bucket) = by_bits.get_mut(&bb) {
            for pi in bucket.iter() {
                // tentative
                let new_qlog = qlog + pi.log2;
                if new_qlog <= limit_log2 + 1e-12 {
                    sel.push(pi.clone());
                    q *= &pi.value;
                    qlog = new_qlog;
                }
            }
        }
    }

    sel
}
