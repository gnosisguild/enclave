// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_events::SequenceIndex;
use std::collections::BTreeMap;

pub struct InMemSequenceIndex {
    index: BTreeMap<u128, u64>,
}

impl InMemSequenceIndex {
    pub fn new() -> Self {
        Self {
            index: BTreeMap::new(),
        }
    }
}

impl SequenceIndex for InMemSequenceIndex {
    fn seek_for_prev(&self, key: u128) -> Result<Option<u64>> {
        // Find the largest key <= the given key and return its value
        Ok(self.index.range(..=key).next_back().map(|(_, &v)| v))
    }

    fn insert(&mut self, key: u128, value: u64) -> Result<()> {
        self.index.insert(key, value);
        Ok(())
    }

    fn get(&self, key: u128) -> Result<Option<u64>> {
        Ok(self.index.get(&key).copied())
    }
}
