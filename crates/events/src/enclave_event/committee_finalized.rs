// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::E3id;
use actix::Message;
use alloy::primitives::U256;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Message, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CommitteeFinalized {
    pub e3_id: E3id,
    pub committee: Vec<String>,
    pub scores: Vec<String>,
    pub chain_id: u64,
}

impl CommitteeFinalized {
    /// Sort committee members by ascending score so every node derives the same
    /// deterministic ordering. The node with the lowest score ends up at index 0.
    /// If scores are empty or unparseable, the order is left unchanged.
    pub fn sort_by_score(&mut self) {
        if self.scores.len() != self.committee.len() || self.scores.is_empty() {
            return;
        }

        // Build (index, parsed_score) pairs
        let mut indices: Vec<usize> = (0..self.committee.len()).collect();
        let parsed: Vec<Option<U256>> =
            self.scores.iter().map(|s| s.parse::<U256>().ok()).collect();

        // If any score fails to parse, leave order unchanged
        if parsed.iter().any(|s| s.is_none()) {
            return;
        }

        indices.sort_by_key(|&i| parsed[i].unwrap());

        let sorted_committee: Vec<String> =
            indices.iter().map(|&i| self.committee[i].clone()).collect();
        let sorted_scores: Vec<String> = indices.iter().map(|&i| self.scores[i].clone()).collect();

        self.committee = sorted_committee;
        self.scores = sorted_scores;
    }
}

impl Display for CommitteeFinalized {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
