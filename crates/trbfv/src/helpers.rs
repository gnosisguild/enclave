// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_crypto::{Cipher, SensitiveBytes};
use fhe::{
    bfv::{self, BfvParameters},
    trbfv::{SmudgingBoundCalculator, SmudgingBoundCalculatorConfig},
};
use fhe_math::rq::Poly;
use fhe_traits::{DeserializeWithContext, Serialize};
use num_bigint::BigUint;
use petname::Petnames;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

pub fn try_poly_from_bytes(bytes: &[u8], params: &BfvParameters) -> Result<Poly> {
    Ok(Poly::from_bytes(bytes, params.ctx_at_level(0)?)?)
}

pub fn try_poly_from_sensitive_bytes(
    bytes: SensitiveBytes,
    params: Arc<BfvParameters>,
    cipher: &Cipher,
) -> Result<Poly> {
    try_poly_from_bytes(&bytes.access(cipher)?, &params)
}

pub fn try_polys_from_sensitive_bytes_vec(
    bytes_vec: Vec<SensitiveBytes>,
    params: Arc<BfvParameters>,
    cipher: &Cipher,
) -> Result<Vec<Poly>> {
    bytes_vec
        .into_iter()
        .map(|s| try_poly_from_sensitive_bytes(s, params.clone(), cipher))
        .collect::<Result<Vec<_>>>()
}

pub fn calculate_error_size(
    params: Arc<bfv::BfvParameters>,
    n: usize,
    num_ciphertexts: usize,
) -> Result<BigUint> {
    let config = SmudgingBoundCalculatorConfig::new(params, n, num_ciphertexts);
    let calculator = SmudgingBoundCalculator::new(config);
    Ok(calculator.calculate_sm_bound()?)
}

pub fn stringify_poly(name: &str, poly: &Poly) -> String {
    format!("{}=Poly({})", name, hash_to_petname(hash_poly(poly)))
}

pub fn print_poly(name: &str, poly: &Poly) {
    println!("{}", stringify_poly(name, poly));
}

fn hash_to_petname(hash: u64) -> String {
    let petnames = Petnames::default();

    // Access as fields, not methods
    let adjectives = &petnames.adjectives;
    let nouns = &petnames.nouns;

    let adj_idx = (hash % adjectives.len() as u64) as usize;
    let noun_idx = ((hash / adjectives.len() as u64) % nouns.len() as u64) as usize;

    format!("{}-{}", adjectives[adj_idx], nouns[noun_idx])
}

fn hash_poly(poly: &Poly) -> u64 {
    hash_bytes(&poly.to_bytes())
}

fn hash_bytes(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}
