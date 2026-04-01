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
/// The index is eagerly rebuilt when the struct is deserialized.
///
/// `PartialEq` compares only the `members` vec (the canonical data);
/// the `index` is a derived cache.
#[derive(Clone, Debug, Serialize)]
pub struct Committee {
    /// Ordered member list — index == party_id.
    members: Vec<String>,
    /// Lowercased-address → party_id for O(1) lookup.
    #[serde(skip)]
    index: HashMap<String, u64>,
}

impl PartialEq for Committee {
    fn eq(&self, other: &Self) -> bool {
        self.members == other.members
    }
}

impl Eq for Committee {}

impl<'de> Deserialize<'de> for Committee {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Inner {
            members: Vec<String>,
        }
        let inner = Inner::deserialize(deserializer)?;
        Ok(Committee::new(inner.members))
    }
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

    pub fn is_active_aggregator(&self, my_addr: &str, expelled: &[u64]) -> bool {
        (0..self.members.len() as u64)
            .find(|party_id| !expelled.contains(party_id))
            .and_then(|party_id| self.members.get(party_id as usize))
            .map(|addr| addr.eq_ignore_ascii_case(my_addr))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::Committee;

    #[test]
    fn picks_lowest_non_expelled_party_in_sorted_committee_as_aggregator() {
        // Committee order is already score-sorted before it is stored.
        let committee = Committee::new(vec![
            "0xbbb".to_string(),
            "0xccc".to_string(),
            "0xaaa".to_string(),
        ]);

        assert!(committee.is_active_aggregator("0xBbB", &[]));
        assert!(committee.is_active_aggregator("0xccc", &[0]));
        assert!(committee.is_active_aggregator("0xaaa", &[0, 1]));
        assert!(!committee.is_active_aggregator("0xaaa", &[0, 1, 2]));
    }
}
