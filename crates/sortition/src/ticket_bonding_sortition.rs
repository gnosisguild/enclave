// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::primitives::{keccak256, Address};
use anyhow::{anyhow, Result};
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

/// A node with its available tickets (after subtracting active jobs)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeWithTickets {
    pub address: Address,
    pub available_tickets: u64,
}

/// A winning ticket with its score
#[derive(Clone, Debug)]
pub struct WinningTicket {
    pub address: Address,
    pub ticket_number: u64,
    pub score: BigUint,
}

/// Ticket-based sortition using bonding registry state
///
/// Algorithm:
/// 1. Each node has N available tickets (ticket_balance / ticket_price - active_jobs)
/// 2. For each ticket of each node, compute hash(node_address, ticket_number, e3_id, seed)
/// 3. Find the lowest score ticket for each node
/// 4. Sort all nodes by their lowest ticket score
/// 5. Select the top M nodes
pub struct TicketBondingSortition {
    /// Desired committee size
    pub size: usize,
}

impl TicketBondingSortition {
    pub fn new(size: usize) -> Self {
        Self { size }
    }

    /// Compute the score for a specific ticket
    ///
    /// Score = keccak256(node_address || ticket_number || e3_id || seed)
    fn compute_ticket_score(
        node_address: Address,
        ticket_number: u64,
        e3_id: u64,
        seed: u64,
    ) -> BigUint {
        let mut message = Vec::with_capacity(20 + 8 + 8 + 8);
        message.extend_from_slice(node_address.as_slice());
        message.extend_from_slice(&ticket_number.to_be_bytes());
        message.extend_from_slice(&e3_id.to_be_bytes());
        message.extend_from_slice(&seed.to_be_bytes());
        
        let hash = keccak256(&message);
        BigUint::from_bytes_be(&hash.0)
    }

    /// Find the lowest scoring ticket for a given node
    fn find_best_ticket_for_node(
        node: &NodeWithTickets,
        e3_id: u64,
        seed: u64,
    ) -> Option<WinningTicket> {
        if node.available_tickets == 0 {
            return None;
        }

        let mut best: Option<WinningTicket> = None;

        for ticket_num in 1..=node.available_tickets {
            let score = Self::compute_ticket_score(node.address, ticket_num, e3_id, seed);

            match &best {
                None => {
                    best = Some(WinningTicket {
                        address: node.address,
                        ticket_number: ticket_num,
                        score,
                    });
                }
                Some(current_best) => {
                    if score < current_best.score
                        || (score == current_best.score && ticket_num < current_best.ticket_number)
                    {
                        best = Some(WinningTicket {
                            address: node.address,
                            ticket_number: ticket_num,
                            score,
                        });
                    }
                }
            }
        }

        best
    }

    /// Determine the committee from a list of nodes with their available tickets
    ///
    /// Returns the sorted list of winning nodes (top M by lowest ticket score)
    pub fn get_committee(
        &self,
        nodes: &[NodeWithTickets],
        e3_id: u64,
        seed: u64,
    ) -> Result<Vec<Address>> {
        if nodes.is_empty() || self.size == 0 {
            return Ok(Vec::new());
        }

        // Find the best ticket for each node
        let mut winning_tickets: Vec<WinningTicket> = nodes
            .iter()
            .filter_map(|node| Self::find_best_ticket_for_node(node, e3_id, seed))
            .collect();

        if winning_tickets.is_empty() {
            return Err(anyhow!("No nodes with available tickets"));
        }

        // Sort by score (ascending), then by ticket number if scores are equal
        winning_tickets.sort_unstable_by(|a, b| {
            a.score
                .cmp(&b.score)
                .then(a.ticket_number.cmp(&b.ticket_number))
        });

        // Select top M nodes
        let selected_size = self.size.min(winning_tickets.len());
        Ok(winning_tickets
            .into_iter()
            .take(selected_size)
            .map(|w| w.address)
            .collect())
    }

    /// Check if a specific node is in the committee
    pub fn is_node_in_committee(
        &self,
        nodes: &[NodeWithTickets],
        e3_id: u64,
        seed: u64,
        target_address: Address,
    ) -> Result<bool> {
        let committee = self.get_committee(nodes, e3_id, seed)?;
        Ok(committee.contains(&target_address))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::keccak256;

    fn address(i: u64) -> Address {
        let h = keccak256([b"addr".as_slice(), &i.to_be_bytes()].concat());
        let mut bytes20 = [0u8; 20];
        bytes20.copy_from_slice(&h.0[12..32]);
        Address::from(bytes20)
    }

    #[test]
    fn test_ticket_bonding_sortition() {
        let nodes = vec![
            NodeWithTickets {
                address: address(1),
                available_tickets: 5,
            },
            NodeWithTickets {
                address: address(2),
                available_tickets: 3,
            },
            NodeWithTickets {
                address: address(3),
                available_tickets: 7,
            },
            NodeWithTickets {
                address: address(4),
                available_tickets: 2,
            },
            NodeWithTickets {
                address: address(5),
                available_tickets: 0, // No available tickets
            },
        ];

        let sortition = TicketBondingSortition::new(3);
        let e3_id = 12345;
        let seed = 0xABCDEF;

        let committee = sortition
            .get_committee(&nodes, e3_id, seed)
            .expect("Should get committee");

        assert_eq!(committee.len(), 3);
        println!("Committee: {:?}", committee);

        // Verify the committee is deterministic
        let committee2 = sortition
            .get_committee(&nodes, e3_id, seed)
            .expect("Should get committee");
        assert_eq!(committee, committee2);

        // Verify node with 0 tickets is not selected
        assert!(!committee.contains(&address(5)));
    }

    #[test]
    fn test_is_node_in_committee() {
        let nodes = vec![
            NodeWithTickets {
                address: address(1),
                available_tickets: 5,
            },
            NodeWithTickets {
                address: address(2),
                available_tickets: 3,
            },
            NodeWithTickets {
                address: address(3),
                available_tickets: 7,
            },
        ];

        let sortition = TicketBondingSortition::new(2);
        let e3_id = 12345;
        let seed = 0xABCDEF;

        let committee = sortition
            .get_committee(&nodes, e3_id, seed)
            .expect("Should get committee");

        for node in &nodes {
            let is_in = sortition
                .is_node_in_committee(&nodes, e3_id, seed, node.address)
                .expect("Should check membership");
            assert_eq!(
                is_in,
                committee.contains(&node.address),
                "Membership check should match committee"
            );
        }
    }

    #[test]
    fn test_active_jobs_penalty() {
        // Node 1 has more tickets but they're reduced by active jobs
        let nodes = vec![
            NodeWithTickets {
                address: address(1),
                available_tickets: 10, // e.g., had 15 tickets but 5 active jobs
            },
            NodeWithTickets {
                address: address(2),
                available_tickets: 10, // e.g., had 10 tickets but 0 active jobs
            },
        ];

        let sortition = TicketBondingSortition::new(1);
        let e3_id = 12345;
        let seed = 0xABCDEF;

        let committee = sortition
            .get_committee(&nodes, e3_id, seed)
            .expect("Should get committee");

        assert_eq!(committee.len(), 1);
        // Both have same available tickets, result is deterministic based on scores
    }
}

