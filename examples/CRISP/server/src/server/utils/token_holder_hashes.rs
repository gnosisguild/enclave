// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use ark_bn254::Fr;
use ark_ff::{BigInt, BigInteger};
use light_poseidon::{Poseidon, PoseidonHasher};
use num_bigint::BigUint;
use std::str::FromStr;

use super::TokenHolder;

/// Computes Poseidon hashes for token holder address + balance pairs.
///
/// # Arguments
/// * `token_holders` - A vector of TokenHolder structs containing address and balance.
///
/// # Returns
/// A vector of hex-encoded Poseidon hashes, one for each token holder.
pub fn compute_token_holder_hashes(token_holders: &[TokenHolder]) -> Vec<String> {
    let mut hashes = Vec::new();

    for holder in token_holders {
        // Convert address directly to field element.
        let address_hex = holder.address.trim_start_matches("0x");
        let address_bigint = BigUint::parse_bytes(address_hex.as_bytes(), 16).unwrap();
        let address_fr = Fr::from_str(&address_bigint.to_string()).unwrap();

        // Convert balance to field element.
        let balance_fr = Fr::from_str(&holder.balance).unwrap();

        // Compute Poseidon hash of address + balance.
        let mut poseidon_instance = Poseidon::<Fr>::new_circom(2).unwrap();
        let hash_bigint: BigInt<4> = poseidon_instance
            .hash(&[address_fr, balance_fr])
            .unwrap()
            .into();
        let hash = hex::encode(hash_bigint.to_bytes_be());

        hashes.push(hash);
    }

    hashes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_token_holder_hashes() {
        let token_holders = vec![
            TokenHolder {
                address: "0x1234567890123456789012345678901234567890".to_string(),
                balance: "1000".to_string(),
            },
            TokenHolder {
                address: "0x2345678901234567890123456789012345678901".to_string(),
                balance: "500".to_string(),
            },
        ];

        let hashes = compute_token_holder_hashes(&token_holders);

        println!("Hashes: {:?}", hashes);

        assert_eq!(hashes.len(), 2);
        assert!(!hashes[0].is_empty());
        assert!(!hashes[1].is_empty());
        assert_ne!(hashes[0], hashes[1]);
        assert_eq!(
            hashes[0],
            "0cb36cd64fcc99d7f742ae77954eda75236e182d7c10de1660f62f56c582b518"
        );
        assert_eq!(
            hashes[1],
            "0793d785764e7afa3343e9ef2f1b1ad6d367a93622ddaaec328686a402a1d085"
        );

        // Verify hash format (should be hex string)
        for hash in &hashes {
            assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}
