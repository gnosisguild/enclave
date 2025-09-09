// SPDX-License-Identifier: LGPL-3.0-only

use crate::ticket::WinnerTicket;
use anyhow::Result;
use std::collections::HashMap;

/// Committee selection based on ticket scores.
///
/// Given a set of `WinnerTicket`s (one or more per node), this
/// algorithm collapses them to at most one ticket per node,
/// sorts by `(score, ticket_id)`, and returns the top-N.
pub struct ScoreSortition {
    /// Desired committee size (N).
    pub size: usize,
}

impl ScoreSortition {
    /// Create a new `ScoreSortition` instance for a committee of given size.
    pub fn new(size: usize) -> Self {
        Self { size }
    }

    /// Compute the committee from submitted winner tickets.
    ///
    /// # Behavior
    /// - If multiple tickets from the same address are present,
    ///   only the one with the lowest `(score, ticket_id)` is kept.
    /// - The remaining tickets are sorted in ascending order by
    ///   `(score, ticket_id)`.
    /// - The top-`size` tickets are returned as the committee.
    ///
    /// # Parameters
    /// - `submissions`: candidate winner tickets from all registered nodes.
    ///
    /// # Returns
    /// - `Ok(Vec<WinnerTicket>)` with the selected committee (length ≤ `size`).
    /// - An empty vector if no submissions are provided or `size == 0`.
    pub fn get_committee(&self, submissions: &[WinnerTicket]) -> Result<Vec<WinnerTicket>> {
        if submissions.is_empty() || self.size == 0 {
            return Ok(Vec::new());
        }

        // Keep best ticket per address
        let mut best_map: HashMap<[u8; 20], WinnerTicket> =
            HashMap::with_capacity(submissions.len());
        for s in submissions {
            let key = s.address.0 .0;
            match best_map.get_mut(&key) {
                None => {
                    best_map.insert(key, s.clone());
                }
                Some(cur) => {
                    if s.score < cur.score || (s.score == cur.score && s.ticket_id < cur.ticket_id)
                    {
                        *cur = s.clone();
                    }
                }
            }
        }

        let mut items: Vec<WinnerTicket> = best_map.into_values().collect();

        // Sort ascending by (score, ticket_id)
        items.sort_unstable_by(|a, b| a.score.cmp(&b.score).then(a.ticket_id.cmp(&b.ticket_id)));

        // Take top-N
        let k = self.size.min(items.len());
        items.truncate(k);
        Ok(items)
    }
}
