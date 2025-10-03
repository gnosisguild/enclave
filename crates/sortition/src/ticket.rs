// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::primitives::{keccak256, Address};
use anyhow::{anyhow, Result};
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

/// Represents a registered node, identified by its address,
/// and carrying a vector of tickets.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisteredNode {
    pub address: Address,
    pub tickets: Vec<Ticket>,
}

/// A single ticket belonging to a node.
/// Ticket IDs are assumed to be globally unique.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ticket {
    pub ticket_id: u64,
}

/// A node’s candidate for committee selection.
/// Chosen as the ticket with the lowest score among all of its tickets.
#[derive(Clone, Debug)]
pub struct WinnerTicket {
    pub address: Address,
    pub ticket_id: u64,
    /// Deterministic score, derived from hashing seed, address, and ticket ID.
    /// Lower is considered better.
    pub score: BigUint,
}

/// Build the message to hash for a ticket’s score.
///
/// Message format:
/// `b"ticket_score" || seed_be64 || address(20B) || ticket_id_be64`
fn serialize_message(seed: u64, address: Address, ticket_id: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(12 + 8 + 20 + 8);
    buf.extend_from_slice(b"ticket_score"); // domain separation tag
    buf.extend_from_slice(&seed.to_be_bytes());
    buf.extend_from_slice(address.as_slice());
    buf.extend_from_slice(&ticket_id.to_be_bytes());
    buf
}

/// Compute a deterministic score for a ticket.
///
/// The score is defined as the Keccak-256 hash of the domain-separated
/// message, interpreted as a big-endian integer.
pub fn hash_to_score(seed: u64, address: Address, ticket_id: u64) -> BigUint {
    let msg = serialize_message(seed, address, ticket_id);
    let digest = keccak256(&msg);
    BigUint::from_bytes_be(&digest.0)
}

/// For a given node, compute the score for each ticket and
/// return the ticket with the lowest score.
///
/// Ties are broken deterministically by selecting the ticket
/// with the smaller ticket ID.
///
/// # Errors
/// Returns an error if the node has no tickets.
pub fn calculate_best_ticket_for_node(
    seed: u64,
    registered_node: &RegisteredNode,
) -> Result<WinnerTicket> {
    if registered_node.tickets.is_empty() {
        return Err(anyhow!("no tickets in the registered node"));
    }

    let mut best: Option<WinnerTicket> = None;

    for ticket in &registered_node.tickets {
        let score = hash_to_score(seed, registered_node.address, ticket.ticket_id);

        match &best {
            None => {
                best = Some(WinnerTicket {
                    address: registered_node.address,
                    ticket_id: ticket.ticket_id,
                    score,
                });
            }
            Some(cur) => {
                if score < cur.score || (score == cur.score && ticket.ticket_id < cur.ticket_id) {
                    best = Some(WinnerTicket {
                        address: registered_node.address,
                        ticket_id: ticket.ticket_id,
                        score,
                    });
                }
            }
        }
    }
    // TODO: Add sign to the best ticket, so it can be verified

    best.ok_or_else(|| anyhow!("no winning ticket found"))
}
