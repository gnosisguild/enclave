// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::ticket::{calculate_best_ticket_for_node, RegisteredNode, WinnerTicket};
use alloy::primitives::Address;
use anyhow::Result;
use std::collections::{hash_map::Entry, HashMap};

/// Deterministic committee selection based on ticket scores.
///
/// This struct encapsulates the logic to:
/// - Compute each node’s lowest-score ticket using the given round `seed`.
/// - Ensure only one winning ticket per node is considered.
/// - Sort all node winners globally by `(score, ticket_id)` in ascending order.
/// - Select the top `N` nodes as the final committee.
pub struct ScoreSortition {
    /// Desired committee size (N).
    pub size: usize,
}

impl ScoreSortition {
    /// Construct a new `ScoreSortition` for a given committee size.
    pub fn new(size: usize) -> Self {
        Self { size }
    }

    /// Determine the top-N committee members from a list of registered nodes.
    ///
    /// # Parameters
    /// - `seed`: Round seed used for deterministic ticket scoring.
    /// - `nodes`: Snapshot of all registered nodes (each with its tickets).
    ///
    /// # Returns
    /// A sorted vector of `WinnerTicket`s representing the selected committee.
    /// Returns an empty vector if no nodes have tickets or if `size == 0`.
    pub fn get_committee(&self, seed: u64, nodes: &[RegisteredNode]) -> Result<Vec<WinnerTicket>> {
        if nodes.is_empty() || self.size == 0 {
            return Ok(Vec::new());
        }

        let mut best_map: HashMap<Address, WinnerTicket> = HashMap::with_capacity(nodes.len());

        for n in nodes {
            if n.tickets.is_empty() {
                continue;
            }

            let w = calculate_best_ticket_for_node(seed, n)?;
            match best_map.entry(w.address) {
                Entry::Vacant(v) => {
                    v.insert(w);
                }
                Entry::Occupied(mut o) => {
                    let cur = o.get_mut();
                    if w.score < cur.score || (w.score == cur.score && w.ticket_id < cur.ticket_id)
                    {
                        *cur = w;
                    }
                }
            }
        }

        let mut items: Vec<WinnerTicket> = best_map.into_values().collect();

        // Sort ascending by (score, ticket_id)
        items.sort_unstable_by(|a, b| a.score.cmp(&b.score).then(a.ticket_id.cmp(&b.ticket_id)));

        let k = self.size.min(items.len());
        items.truncate(k);
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use crate::ticket::{RegisteredNode, Ticket, WinnerTicket};
    use crate::ticket_sortition::ScoreSortition;
    use alloy::primitives::{keccak256, Address};
    use std::collections::HashSet;

    fn ticket_count(i: u64) -> u64 {
        let h = keccak256([b"tickets".as_slice(), &i.to_be_bytes()].concat());
        (u128::from_be_bytes(h.0[0..16].try_into().unwrap()) % 7 + 1) as u64
    }

    fn address(i: u64) -> Address {
        let h = keccak256([b"addr".as_slice(), &i.to_be_bytes()].concat());
        let mut bytes20 = [0u8; 20];
        bytes20.copy_from_slice(&h.0[12..32]);
        Address::from(bytes20)
    }

    fn build_nodes() -> Vec<RegisteredNode> {
        let mut next_ticket_id: u64 = 1;
        (0u64..10)
            .map(|i| {
                let address = address(i);
                let t = ticket_count(i);
                let mut tickets = Vec::with_capacity(t as usize);
                for _ in 0..t {
                    tickets.push(Ticket {
                        ticket_id: next_ticket_id,
                    });
                    next_ticket_id += 1;
                }
                RegisteredNode { address, tickets }
            })
            .collect()
    }

    #[test]
    fn test_ticket_sortition() {
        let seed: u64 = 0xA1B2_C3D4_E5F6_7789;
        let committee_size: usize = 3;

        let nodes = build_nodes();
        assert_eq!(nodes.len(), 10);
        assert!(nodes.iter().all(|n| !n.tickets.is_empty()));

        println!("NODES {:#?}", nodes);

        let mut all_ids = HashSet::new();
        for n in &nodes {
            for t in &n.tickets {
                assert!(all_ids.insert(t.ticket_id));
            }
        }

        let committee: Vec<WinnerTicket> = ScoreSortition::new(committee_size)
            .get_committee(seed, &nodes)
            .expect("score sortition should succeed");
        assert_eq!(committee.len(), committee_size);

        // Check winners deterministically for the given seed
        assert_eq!(committee[0].address, nodes[9].address);
        assert_eq!(committee[1].address, nodes[1].address);
        assert_eq!(committee[2].address, nodes[0].address);

        println!("COMMITTEE {:#?}", committee);
    }
}
