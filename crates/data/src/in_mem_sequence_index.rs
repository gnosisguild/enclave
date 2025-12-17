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
    /// Creates a new InMemSequenceIndex with an empty in-memory index.
    ///
    /// # Examples
    ///
    /// ```
    /// let idx = InMemSequenceIndex::new();
    /// // new index contains no entries
    /// assert!(idx.get(1).unwrap().is_none());
    /// ```
    pub fn new() -> Self {
        Self {
            index: BTreeMap::new(),
        }
    }
}

impl SequenceIndex for InMemSequenceIndex {
    /// Finds the value for the smallest stored key that is greater than or equal to `key`.
    ///
    /// Returns the value associated with the smallest stored key >= `key`, or `None` if no such key exists.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut idx = InMemSequenceIndex::new();
    /// idx.insert(100, 1).unwrap();
    /// idx.insert(200, 2).unwrap();
    /// assert_eq!(idx.seek(50).unwrap(), Some(1));
    /// assert_eq!(idx.seek(150).unwrap(), Some(2));
    /// assert_eq!(idx.seek(999).unwrap(), None);
    /// ```
    fn seek(&self, key: u128) -> Result<Option<u64>> {
        Ok(self.index.range(key..).next().map(|(_, &v)| v))
    }

    /// Inserts or updates the in-memory mapping from a 128-bit key to a 64-bit sequence index.
    ///
    /// The method stores the provided `value` under `key`, replacing any existing entry.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut idx = InMemSequenceIndex::new();
    /// idx.insert(100u128, 1u64).unwrap();
    /// assert_eq!(idx.get(100u128).unwrap(), Some(1));
    /// ```
    fn insert(&mut self, key: u128, value: u64) -> Result<()> {
        self.index.insert(key, value);
        Ok(())
    }

    /// Retrieves the sequence value associated with the exact `key`, if present.
    ///
    /// # Returns
    ///
    /// `Some(value)` containing the stored sequence value when the key exists, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut idx = InMemSequenceIndex::new();
    /// idx.insert(100, 1).unwrap();
    /// assert_eq!(idx.get(100).unwrap(), Some(1));
    /// assert_eq!(idx.get(50).unwrap(), None);
    /// ```
    fn get(&self, key: u128) -> Result<Option<u64>> {
        Ok(self.index.get(&key).copied())
    }
}

#[cfg(test)]
mod tests {
    use crate::InMemSequenceIndex;
    use e3_events::SequenceIndex;

    #[test]
    fn seek_finds_nearest_key_at_or_after_target() {
        let mut index = InMemSequenceIndex::new();
        index.insert(100, 1).unwrap();
        index.insert(200, 2).unwrap();
        index.insert(300, 3).unwrap();

        assert_eq!(index.seek(50).unwrap(), Some(1));

        // Exact matches
        assert_eq!(index.seek(100).unwrap(), Some(1));
        assert_eq!(index.seek(200).unwrap(), Some(2));
        assert_eq!(index.seek(300).unwrap(), Some(3));

        // Between keys (returns next)
        assert_eq!(index.seek(150).unwrap(), Some(2));
        assert_eq!(index.seek(250).unwrap(), Some(3));

        assert_eq!(index.seek(999).unwrap(), None);
    }
}