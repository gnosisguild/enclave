// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use ark_bn254::Fr;
use ark_ff::{BigInt, BigInteger};
use light_poseidon::{Poseidon, PoseidonHasher};
use num_bigint::BigUint;
use num_traits::Num;
use sha3::{Digest, Keccak256};
use zk_kit_imt::imt::IMT;
use std::str::FromStr;

pub struct MerkleTree {
    pub leaf_hashes: Vec<String>,
    pub arity: usize, 
    pub zero_value: String,
    pub depth: usize,
}

impl MerkleTree {
    pub fn new() -> Self {
        Self {
            leaf_hashes: Vec::new(),
            arity: 0,
            zero_value: 0.to_string(),
            depth: 0,
        }
    }

    pub fn with_defaults(mut self) -> Self {
        self.arity = 2;
        self.zero_value = "0".to_string();
        self.depth = 20;
        self
    }   

    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    pub fn with_arity(mut self, arity: usize) -> Self {
        self.arity = arity;
        self
    }

    pub fn with_zero_value(mut self, zero_value: String) -> Self {
        self.zero_value = zero_value;
        self
    }

    pub fn with_leaf_hashes(mut self, leaf_hashes: Vec<String>) -> Self {
        self.leaf_hashes = leaf_hashes;
        self
    }

    pub fn compute_leaf_hashes(&mut self, data: &[(Vec<u8>, u64)]) {
        for item in data {
            let hex_output = hex::encode(Keccak256::digest(&item.0));
            let sanitized_hex = hex_output.trim_start_matches("0x");
            let numeric_value = BigUint::from_str_radix(sanitized_hex, 16)
                .unwrap()
                .to_string();
            let fr_element = Fr::from_str(&numeric_value).unwrap();
            let index_element = Fr::from_str(&item.1.to_string()).unwrap();
            let mut poseidon_instance = Poseidon::<Fr>::new_circom(2).unwrap();
            let hash_bigint: BigInt<4> = poseidon_instance
                .hash(&[fr_element, index_element])
                .unwrap()
                .into();
            let hash = hex::encode(hash_bigint.to_bytes_be());
            self.leaf_hashes.push(hash);
        }
    }

    fn poseidon_hash(nodes: Vec<String>) -> String {
        let mut poseidon = Poseidon::<Fr>::new_circom(2).unwrap();
        let mut field_elements = Vec::new();

        for node in nodes {
            let sanitized_node = node.trim_start_matches("0x");
            let numeric_str = BigUint::from_str_radix(sanitized_node, 16)
                .unwrap()
                .to_string();
            let field_repr = Fr::from_str(&numeric_str).unwrap();
            field_elements.push(field_repr);
        }

        let result_hash: BigInt<4> = poseidon.hash(&field_elements).unwrap().into();
        hex::encode(result_hash.to_bytes_be())
    }

    pub fn build_tree(&self) -> IMT {
        let mut tree = IMT::new(Self::poseidon_hash, self.depth, self.zero_value.clone(), self.arity, vec![]).unwrap();
        for leaf in &self.leaf_hashes {
            tree.insert(leaf.clone()).unwrap();
        }

        tree 
    }
}
