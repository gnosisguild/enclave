// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use rand::{rngs::StdRng, Rng, SeedableRng};

pub struct IndexSortition {
    pub random_seed: u64,
    pub num_nodes: usize,
    pub size: usize,
}

impl IndexSortition {
    pub fn new(random_seed: u64, num_nodes: usize, size: usize) -> Self {
        Self {
            random_seed,
            num_nodes,
            size,
        }
    }

    fn _get_committee(&mut self) -> Vec<usize> {
        // Initialize a vector with indices of nodes as elements
        let mut leaf_indices: Vec<usize> = (0..self.num_nodes).collect();
        // Initialize an empty vector to store the committee
        let mut committee: Vec<usize> = Vec::new();

        // Initialize the random number generator with the given `seed`
        let mut rng = StdRng::seed_from_u64(self.random_seed);

        // Partial shuffle for only the `committee_size` number of nodes
        for _ in 0..self.size {
            // Choose a random leaf index from the `leaf_indices`
            let j = rng.gen_range(0..leaf_indices.len());
            // Push the chosen leaf index to the `committee`
            committee.push(leaf_indices[j]);
            // Remove the chosen leaf index from the `leaf_indices`
            leaf_indices.remove(j);
        }

        // Return the leaf indices of the selected committee
        committee
    }
}
