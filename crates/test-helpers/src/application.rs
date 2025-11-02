// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.;

//! Test Application
//! The following simulates a user application for testing
use rand::{distributions::Uniform, prelude::Distribution, thread_rng};

use fhe_traits::{FheEncoder, FheEncrypter};
use std::sync::Arc;

use fhe::bfv::{self, Ciphertext, Encoding, Plaintext, PublicKey};

/// Each Voter encrypts `num_votes_per_voter` random bits and returns the ciphertexts along with
/// the underlying plaintexts for verification.
pub fn generate_ciphertexts(
    pk: &PublicKey,
    params: Arc<bfv::BfvParameters>,
    num_voters: usize,
    num_votes_per_voter: usize,
) -> (Vec<Vec<Ciphertext>>, Vec<Vec<u64>>) {
    let dist = Uniform::new_inclusive(0, 1);
    let mut rng = thread_rng();
    println!("generating ciphertexts...");
    let numbers: Vec<Vec<u64>> = (0..num_voters)
        .map(|_| {
            (0..num_votes_per_voter)
                .map(|_| dist.sample(&mut rng))
                .collect()
        })
        .collect();
    println!("Encrypting ciphertexts...");
    let nl = numbers.len();
    let ciphertexts: Vec<Vec<Ciphertext>> = numbers
        .iter()
        .enumerate()
        .map(|(ni, vals)| {
            let mut rng = thread_rng();
            vals.iter()
                .enumerate()
                .map(|(i, &val)| {
                    println!("Encrypting {}/{}/{}", i, ni, nl);
                    let pt = Plaintext::try_encode(&[val], Encoding::poly(), &params).unwrap();
                    pk.try_encrypt(&pt, &mut rng).unwrap()
                })
                .collect()
        })
        .collect();
    println!("Finished encryption!");
    (ciphertexts, numbers)
}

/// Tally the submitted ciphertexts column-wise to produce aggregated sums.
pub fn run_application(
    ciphertexts: &[Vec<Ciphertext>],
    params: Arc<bfv::BfvParameters>,
    num_votes_per_voter: usize,
) -> Vec<Ciphertext> {
    println!("Running application");
    if ciphertexts.is_empty() {
        return Vec::new();
    }

    let mut sums: Vec<Ciphertext> = (0..num_votes_per_voter)
        .map(|_| Ciphertext::zero(&params))
        .collect();

    for ct_group in ciphertexts {
        for (j, ciphertext) in ct_group.iter().enumerate() {
            sums[j] += ciphertext;
        }
    }
    sums.into_iter().collect()
}
