// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Ordered list of committee members where index == party_id.
///
/// Provides O(1) address→party_id lookups via an internal index.
/// The index is skipped during serialization and rebuilt lazily on first lookup
/// after deserialization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Committee {
    /// Ordered member list — index == party_id.
    members: Vec<String>,
    /// Lowercased-address → party_id for O(1) lookup.
    #[serde(skip)]
    index: HashMap<String, u64>,
}

impl Committee {
    pub fn new(members: Vec<String>) -> Self {
        let index = members
            .iter()
            .enumerate()
            .map(|(i, addr)| (addr.to_lowercase(), i as u64))
            .collect();
        Self { members, index }
    }

    /// Resolve an address to its party_id (position in the committee list).
    pub fn party_id_for(&self, addr: &str) -> Option<u64> {
        // Rebuild index lazily after deserialization (serde skip)
        if self.index.is_empty() && !self.members.is_empty() {
            // Fallback linear scan when index hasn't been rebuilt
            let lower = addr.to_lowercase();
            return self
                .members
                .iter()
                .position(|a| a.to_lowercase() == lower)
                .map(|i| i as u64);
        }
        self.index.get(&addr.to_lowercase()).copied()
    }

    /// Check if an address is a committee member.
    pub fn contains(&self, addr: &str) -> bool {
        self.party_id_for(addr).is_some()
    }

    /// The ordered member list (index == party_id).
    pub fn members(&self) -> &[String] {
        &self.members
    }

    pub fn len(&self) -> usize {
        self.members.len()
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}
