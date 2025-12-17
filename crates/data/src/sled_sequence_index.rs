// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::path::PathBuf;

use anyhow::{Context, Result};
use e3_events::SequenceIndex;
use sled::Tree;

use crate::sled_utils::{clear_all_caches, get_or_open_db_tree};

pub struct SledSequenceIndex {
    db: Tree,
}

impl SledSequenceIndex {
    /// Creates a new SledSequenceIndex by opening or creating the specified sled tree.
    ///
    /// The `path` identifies the database directory and `tree` is the name of the sled tree to open.
    /// Returns an error if the underlying database tree cannot be opened or created.
    ///
    /// # Examples
    ///
    /// ```
    /// use tempfile::tempdir;
    /// use std::path::PathBuf;
    /// let dir = tempdir().unwrap();
    /// let path = dir.path().to_path_buf();
    /// let idx = sled_sequence_index::SledSequenceIndex::new(&path, "test_tree").unwrap();
    /// ```
    pub fn new(path: &PathBuf, tree: &str) -> Result<Self> {
        let db = get_or_open_db_tree(path, tree)?;
        Ok(Self { db })
    }

    /// Closes all cached sled database connections and releases related resources.
    ///
    /// # Examples
    ///
    /// ```
    /// SledSequenceIndex::close_all_connections();
    /// ```
    pub fn close_all_connections() {
        clear_all_caches()
    }
}

impl SequenceIndex for SledSequenceIndex {
    /// Fetches the sequence value stored for the given 128-bit key, decoding the stored bytes as a big-endian `u64`.
    ///
    /// Returns `Some(u64)` when a value is found for `key`, `None` when the key is absent.
    /// Returns an error if the database operation fails or if the stored value cannot be converted into an 8-byte big-endian `u64`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tempfile::tempdir;
    /// # use crate::sled_sequence_index::SledSequenceIndex;
    /// let dir = tempdir().unwrap();
    /// let mut idx = SledSequenceIndex::new(&dir.path().to_path_buf(), "test_tree").unwrap();
    /// idx.insert(100u128, 1u64).unwrap();
    /// assert_eq!(idx.get(100u128).unwrap(), Some(1));
    /// ```
    fn get(&self, key: u128) -> Result<Option<u64>> {
        self.db
            .get(key.to_be_bytes().to_vec())
            .context(format!("Failed to fetch timestamp: {}", key))?
            .map(|v| Ok(u64::from_be_bytes(v.as_ref().try_into()?)))
            .transpose()
    }

    /// Inserts a mapping from `key` to `value` into the underlying sled tree.
    ///
    /// The `key` is stored as a big-endian `u128` byte sequence and the `value` is stored as a
    /// big-endian `u64` byte sequence.
    ///
    /// # Parameters
    ///
    /// - `key`: Sequence key to insert, encoded as big-endian bytes for storage.
    /// - `value`: Value to associate with `key`, encoded as big-endian bytes for storage.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error with context "Failed to insert key: {key}" if the insert fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use tempfile::tempdir;
    /// use std::path::PathBuf;
    /// use crate::SledSequenceIndex;
    ///
    /// let dir = tempdir().unwrap();
    /// let path: PathBuf = dir.path().to_path_buf();
    /// let mut idx = SledSequenceIndex::new(&path, "doc_tree").unwrap();
    ///
    /// idx.insert(42u128, 7u64).unwrap();
    /// assert_eq!(idx.get(42u128).unwrap(), Some(7u64));
    /// ```
    fn insert(&mut self, key: u128, value: u64) -> Result<()> {
        self.db
            .insert(key.to_be_bytes().to_vec(), value.to_be_bytes().to_vec())
            .context(format!("Failed to insert key: {}", key))?;
        Ok(())
    }

    /// Finds the stored sequence value for the first key at or after `key`.
    ///
    /// Returns the value associated with the smallest stored key greater than or equal to `key`, or `None` if no such key exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the database range query fails or if a found value cannot be converted into an 8-byte `u64`.
    ///
    /// # Examples
    ///
    /// ```
    /// // assuming `idx` is a `SledSequenceIndex` with entries 100 -> 1, 200 -> 2
    /// let found = idx.seek(150).unwrap();
    /// assert_eq!(found, Some(2));
    /// ```
    fn seek(&self, key: u128) -> Result<Option<u64>> {
        let key_bytes = key.to_be_bytes();
        self.db
            .range(key_bytes..)
            .next()
            .transpose()
            .context(format!("Failed to seek: {}", key))?
            .map(|(_, v)| Ok(u64::from_be_bytes(v.as_ref().try_into()?)))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn seek_finds_nearest_key_at_or_after_target() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();

        let mut index = SledSequenceIndex::new(&path, "test_tree").unwrap();

        index.insert(100, 1).unwrap();
        index.insert(200, 2).unwrap();
        index.insert(300, 3).unwrap();

        // Before all keys (returns first)
        assert_eq!(index.seek(50).unwrap(), Some(1));

        // Exact matches
        assert_eq!(index.seek(100).unwrap(), Some(1));
        assert_eq!(index.seek(200).unwrap(), Some(2));
        assert_eq!(index.seek(300).unwrap(), Some(3));

        // Between keys (returns next)
        assert_eq!(index.seek(150).unwrap(), Some(2));
        assert_eq!(index.seek(250).unwrap(), Some(3));

        // After all keys
        assert_eq!(index.seek(999).unwrap(), None);
    }
}