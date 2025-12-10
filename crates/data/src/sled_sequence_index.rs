use std::path::PathBuf;

use anyhow::{Context, Result};
use e3_events::SequenceIndex;
use sled::Tree;

use crate::sled_utils::{clear_all_caches, get_or_open_db_tree};

pub struct SledSequenceIndex {
    db: Tree,
}

impl SledSequenceIndex {
    pub fn new(path: &PathBuf, tree: &str) -> Result<Self> {
        let db = get_or_open_db_tree(path, tree)?;
        Ok(Self { db })
    }

    pub fn close_all_connections() {
        clear_all_caches()
    }
}

impl SequenceIndex for SledSequenceIndex {
    fn get(&self, key: u128) -> Result<Option<u64>> {
        self.db
            .get(key.to_be_bytes().to_vec())
            .context(format!("Failed to fetch timestamp: {}", key))?
            .map(|v| Ok(u64::from_be_bytes(v.as_ref().try_into()?)))
            .transpose()
    }

    fn insert(&mut self, key: u128, value: u64) -> Result<()> {
        Ok(())
    }

    fn seek_for_prev(&self, key: u128) -> Result<Option<u64>> {
        Ok(None)
    }
}
