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
        Ok(None)
    }
    fn insert(&mut self, key: u128, value: u64) -> Result<()> {
        Ok(())
    }
    fn get(&self, key: u128) -> Result<Option<u64>> {
        Ok(None)
    }
}
