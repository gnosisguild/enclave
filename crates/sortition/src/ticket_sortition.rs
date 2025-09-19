// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::ticket::{best_ticket_for_node, RegisteredNode, WinnerTicket};
use anyhow::Result;
use std::collections::HashMap;

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

        let mut best_map: HashMap<[u8; 20], WinnerTicket> = HashMap::with_capacity(nodes.len());

        for n in nodes {
            if n.tickets.is_empty() {
                continue;
            }

            let w = best_ticket_for_node(seed, n)?;
            let key = w.address.0 .0;

            match best_map.get_mut(&key) {
                None => {
                    best_map.insert(key, w);
                }
                Some(cur) => {
                    if w.score < cur.score || (w.score == cur.score && w.ticket_id < cur.ticket_id)
                    {
                        *cur = w;
                    }
                }
            }
        }

        let mut items: Vec<WinnerTicket> = best_map.into_values().collect();

        items.sort_unstable_by(|a, b| a.score.cmp(&b.score).then(a.ticket_id.cmp(&b.ticket_id)));

        let k = self.size.min(items.len());
        items.truncate(k);
        Ok(items)
    }
}
