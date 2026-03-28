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
}
