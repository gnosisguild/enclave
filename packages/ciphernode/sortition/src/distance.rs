use alloy_primitives::{keccak256};
use alloy::{
    primitives::{Address, address},
};
use num::{BigInt, Num};

pub struct DistanceSortition {
    pub random_seed: u64,
    pub registered_nodes: Vec<String>,
    pub size: usize,
}

impl DistanceSortition {
    pub fn new(random_seed: u64, registered_nodes: Vec<String>, size: usize) -> Self {
        Self { random_seed, registered_nodes, size }
    }

    pub fn get_committee(&mut self) -> Vec<(String, String)> {
        let mut scores = self.registered_nodes.iter()
            .map(|address|
                {
                    let concat = address.to_string() + &self.random_seed.to_string();
                    let hash = keccak256(concat).to_string();
                    let without_prefix = hash.trim_start_matches("0x");
                    let z = BigInt::from_str_radix(without_prefix, 16).unwrap();
                    let score = z - BigInt::from(self.random_seed);
                    (score.to_string(), address.to_string())
                })
            .collect::<Vec<(String, String)>>();
            
        scores.sort_by(|a, b| a.0.cmp(&b.0));
        let result = scores[0..self.size].to_vec();
        result
    }
}