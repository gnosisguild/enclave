// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use ark_bn254::Fr;
use ark_ff::{BigInt, BigInteger};
use lean_imt::LeanIMT;
use light_poseidon::{Poseidon, PoseidonHasher};
use num_bigint::BigUint;
use std::str::FromStr;

/// Builds a LeanIMT from a vector of Poseidon hashes.
///
/// # Arguments
/// * `poseidon_hashes` - A vector of hex-encoded Poseidon hashes to use as leaves.
///
/// # Returns
/// A LeanIMT instance with the provided hashes as leaves.
///
/// # Example
/// ```rust
/// use crisp::server::token_holders::build_lean_imt;
///
/// let hashes = vec![
///     "1234567890123456789012345678901234567890123456789012345678901234".to_string(),
///     "5678901234567890123456789012345678901234567890123456789012345678".to_string(),
/// ];
/// let tree = build_lean_imt(hashes);
/// let root = tree.root().unwrap();
/// ```
pub fn build_lean_imt(poseidon_hashes: Vec<String>) -> LeanIMT {
    let mut tree = LeanIMT::new(poseidon_hash);

    // Only insert if we have hashes to avoid empty tree issues
    if !poseidon_hashes.is_empty() {
        tree.insert_many(poseidon_hashes).unwrap();
    }

    tree
}

/// Poseidon hash function for LeanIMT internal nodes.
///
/// # Arguments
/// * `nodes` - A vector of hex-encoded hash strings to hash together.
///
/// # Returns
/// A hex-encoded hash string representing the combined hash.
fn poseidon_hash(nodes: Vec<String>) -> String {
    let mut poseidon = Poseidon::<Fr>::new_circom(2).unwrap();
    let mut field_elements = Vec::new();

    for mut node in nodes {
        if let Some(stripped) = node.strip_prefix("0x") {
            node = stripped.to_string();
        }
        let bigint = BigUint::parse_bytes(node.as_bytes(), 16).unwrap();
        let field_repr = Fr::from_str(&bigint.to_string()).unwrap();
        field_elements.push(field_repr);
    }

    let result_hash: BigInt<4> = poseidon.hash(&field_elements).unwrap().into();
    hex::encode(result_hash.to_bytes_be())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_lean_imt() {
        let hashes = vec![
            "1234567890123456789012345678901234567890123456789012345678901234".to_string(),
            "2345678901234567890123456789012345678901234567890123456789012345".to_string(),
        ];

        let tree = build_lean_imt(hashes);
        let root = tree.root().unwrap();

        assert!(!root.is_empty());
        assert!(root.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_build_lean_imt_empty() {
        let hashes = vec![];
        let _tree = build_lean_imt(hashes);

        // For empty tree, we expect it to be valid but may not have a root.
        // This test just ensures it doesn't panic when creating the tree.
        // The tree should be created successfully even with no leaves.
        assert!(true); // Just verify the tree was created without panicking.
    }
}
