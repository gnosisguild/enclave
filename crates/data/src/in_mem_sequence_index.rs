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
        todo!("do this");
        Ok(None)
    }
    fn insert(&mut self, key: u128, value: u64) -> Result<()> {
        todo!("do this");
        Ok(())
    }
    fn get(&self, key: u128) -> Result<Option<u64>> {
        todo!("do this");
        Ok(None)
    }
}
