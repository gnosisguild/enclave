// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Pure reorg-safety primitives for EVM ingestion.
//!
//! The local event log is append-only and has no truncation primitive, so once
//! a chain log is promoted to an `EnclaveEvent` and folded into state it cannot
//! be cleanly un-applied. The only sound defence against a chain reorg is
//! therefore *prevention*: do not ingest a log until it is buried under enough
//! confirmations that a reorg of that depth is infeasible. This module provides
//! the three pure, deterministic building blocks for that:
//!
//! 1. [`confirmed_head`] / [`is_confirmed`] — the confirmation gate that clamps
//!    how far the reader may advance.
//! 2. [`BlockHashTracker`] — detects a reorg by spotting a previously-seen
//!    block height reappearing with a different hash, reporting the fork point.
//! 3. [`plan_rollback`] — given a detected fork block and the per-aggregate
//!    block cursors, computes which aggregates are affected and the safe block
//!    to resync from.
//!
//! Everything here is clock-free and provider-free so it is fully unit-tested;
//! the actors own the wall clock and provider I/O and feed values in.

use std::collections::HashMap;

/// The highest block height that is safe to ingest given the current chain head
/// and the required confirmation depth. Returns `chain_head` when
/// `confirmations == 0` (no gating), and saturates at 0 for shallow chains.
///
/// A log at height `h` is safe once `h <= chain_head - confirmations`, i.e. once
/// at least `confirmations` blocks have been built on top of it.
pub fn confirmed_head(chain_head: u64, confirmations: u64) -> u64 {
    chain_head.saturating_sub(confirmations)
}

/// Whether a log at `log_block` is confirmed to the required depth relative to
/// `chain_head`.
pub fn is_confirmed(log_block: u64, chain_head: u64, confirmations: u64) -> bool {
    log_block <= confirmed_head(chain_head, confirmations)
}

/// A detected reorg: the chain forked at `fork_block`, meaning every locally
/// applied effect from blocks `>= fork_block` may be orphaned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReorgDetected {
    /// The lowest block height whose hash changed; state at and above this
    /// height must be considered orphaned.
    pub fork_block: u64,
    /// The previously-observed hash at `fork_block`.
    pub old_hash: [u8; 32],
    /// The newly-observed hash at `fork_block`.
    pub new_hash: [u8; 32],
}

/// Tracks `(height -> hash)` observations to detect a reorg the moment a known
/// height is seen with a different hash. Bounded by `capacity`: only the most
/// recent `capacity` heights are retained (older blocks are assumed final).
#[derive(Debug, Clone, Default)]
pub struct BlockHashTracker {
    hashes: HashMap<u64, [u8; 32]>,
    capacity: usize,
}

impl BlockHashTracker {
    /// Create a tracker retaining the most recent `capacity` block hashes.
    /// `capacity` should be at least the reorg-protection depth of interest.
    pub fn new(capacity: usize) -> Self {
        Self {
            hashes: HashMap::new(),
            capacity: capacity.max(1),
        }
    }

    /// Record a block observation. Returns `Some(ReorgDetected)` if `height` was
    /// previously seen with a different hash (a reorg), otherwise `None`.
    pub fn observe(&mut self, height: u64, hash: [u8; 32]) -> Option<ReorgDetected> {
        let detected = match self.hashes.get(&height) {
            Some(prev) if *prev != hash => Some(ReorgDetected {
                fork_block: height,
                old_hash: *prev,
                new_hash: hash,
            }),
            _ => None,
        };

        // Record the new (canonical) hash and evict heights below the retention
        // window so the map stays bounded.
        self.hashes.insert(height, hash);
        if self.hashes.len() > self.capacity {
            if let Some(&max) = self.hashes.keys().max() {
                let cutoff = max.saturating_sub(self.capacity as u64 - 1);
                self.hashes.retain(|&h, _| h >= cutoff);
            }
        }
        detected
    }

    /// Number of retained observations (for diagnostics/tests).
    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }
}

/// A plan describing how to recover from a reorg detected at `fork_block`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackPlan {
    /// The fork point; all state derived from blocks `>= fork_block` is suspect.
    pub fork_block: u64,
    /// The highest block considered safe to keep (`fork_block - 1`, saturating).
    pub resync_from: u64,
    /// Aggregate identifiers whose block cursor is at or beyond the fork and
    /// therefore have locally-applied state that must be re-derived.
    pub affected: Vec<String>,
}

/// Given a detected `fork_block` and the per-aggregate block cursors
/// (`(aggregate_id, highest_applied_block)`), compute which aggregates are
/// affected and the safe block to resync each from.
///
/// Deterministic: the `affected` list is sorted for stable output.
pub fn plan_rollback(fork_block: u64, cursors: &[(String, u64)]) -> RollbackPlan {
    let mut affected: Vec<String> = cursors
        .iter()
        .filter(|(_, applied_block)| *applied_block >= fork_block)
        .map(|(id, _)| id.clone())
        .collect();
    affected.sort();
    RollbackPlan {
        fork_block,
        resync_from: fork_block.saturating_sub(1),
        affected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    #[test]
    fn confirmed_head_clamps_and_saturates() {
        assert_eq!(confirmed_head(100, 0), 100);
        assert_eq!(confirmed_head(100, 12), 88);
        assert_eq!(confirmed_head(5, 12), 0);
    }

    #[test]
    fn is_confirmed_respects_depth() {
        // head 100, depth 12 => safe up to block 88.
        assert!(is_confirmed(88, 100, 12));
        assert!(!is_confirmed(89, 100, 12));
        // depth 0 => everything up to head is confirmed.
        assert!(is_confirmed(100, 100, 0));
    }

    #[test]
    fn tracker_reports_no_reorg_on_fresh_or_consistent_heights() {
        let mut t = BlockHashTracker::new(8);
        assert_eq!(t.observe(10, h(1)), None);
        assert_eq!(t.observe(11, h(2)), None);
        // Re-observing the same height with the same hash is not a reorg.
        assert_eq!(t.observe(10, h(1)), None);
    }

    #[test]
    fn tracker_detects_reorg_on_hash_change() {
        let mut t = BlockHashTracker::new(8);
        t.observe(10, h(1));
        t.observe(11, h(2));
        let d = t.observe(11, h(9)).expect("reorg at 11");
        assert_eq!(
            d,
            ReorgDetected {
                fork_block: 11,
                old_hash: h(2),
                new_hash: h(9),
            }
        );
    }

    #[test]
    fn tracker_evicts_beyond_capacity() {
        let mut t = BlockHashTracker::new(3);
        for height in 1..=10 {
            t.observe(height, h(height as u8));
        }
        // Only the most recent 3 heights retained.
        assert!(t.len() <= 3);
        // An old, evicted height re-observed with a new hash is no longer
        // flagged (assumed final).
        assert_eq!(t.observe(1, h(99)), None);
    }

    #[test]
    fn plan_rollback_selects_affected_aggregates() {
        let cursors = vec![
            ("agg_a".to_string(), 50u64),
            ("agg_b".to_string(), 88u64),
            ("agg_c".to_string(), 120u64),
        ];
        let plan = plan_rollback(90, &cursors);
        assert_eq!(plan.fork_block, 90);
        assert_eq!(plan.resync_from, 89);
        // Only aggregates with cursor >= 90 are affected.
        assert_eq!(plan.affected, vec!["agg_c".to_string()]);
    }

    #[test]
    fn plan_rollback_is_sorted_and_handles_genesis_fork() {
        let cursors = vec![("z".to_string(), 10u64), ("a".to_string(), 10u64)];
        let plan = plan_rollback(0, &cursors);
        assert_eq!(plan.resync_from, 0);
        assert_eq!(plan.affected, vec!["a".to_string(), "z".to_string()]);
    }
}
