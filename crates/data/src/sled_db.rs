// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{Context, Result};
use sled::{transaction::ConflictableTransactionError, Tree};
use std::path::PathBuf;

use crate::{
    sled_utils::{clear_all_caches, get_or_open_db_tree},
    Get, Insert, Remove,
};

pub struct SledDb {
    db: Tree,
}

impl SledDb {
    /// Opens or creates a sled tree at the given filesystem path and returns a `SledDb` wrapping it.
    ///
    /// Errors if the underlying database tree cannot be opened or created.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// let path = std::env::temp_dir().join("sled_db_example");
    /// let db = SledDb::new(&path, "example_tree").unwrap();
    /// ```
    pub fn new(path: &PathBuf, tree: &str) -> Result<Self> {
        let db = get_or_open_db_tree(path, tree)?;
        Ok(Self { db })
    }

    /// Clears all cached sled database trees and connections.
    ///
    /// This closes or clears any in-memory caches used to reuse opened sled trees across the process.
    ///
    /// # Examples
    ///
    /// ```
    /// // Close and clear all cached sled connections.
    /// close_all_connections();
    /// ```
    pub fn close_all_connections() {
        clear_all_caches()
    }

    /// Inserts a key/value pair into the underlying sled tree.
    ///
    /// On success returns `Ok(())`. On failure returns an error with context "Could not insert data into db".
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use crate::sled_db::SledDb;
    /// # use crate::Insert;
    /// let mut db = SledDb::new(&PathBuf::from("/tmp/db"), "default").unwrap();
    /// let msg = Insert::from_parts("my_key".into(), b"my_value".to_vec());
    /// db.insert(msg).unwrap();
    /// ```
    pub fn insert(&mut self, msg: Insert) -> Result<()> {
        self.db
            .insert(msg.key(), msg.value().to_vec())
            .context("Could not insert data into db")?;

        Ok(())
    }

    /// Inserts multiple key/value pairs into the database atomically.
    ///
    /// The provided `msgs` are written inside a single transaction so either all inserts succeed or none are applied.
    ///
    /// # Parameters
    ///
    /// - `msgs`: a slice of `Insert` messages whose `key()` and `value()` are stored.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error with context `"Could not insert batch data into db"` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::{SledDb, Insert};
    /// use std::path::PathBuf;
    ///
    /// let mut db = SledDb::new(&PathBuf::from("/tmp/example"), "tree").unwrap();
    /// let batch = vec![
    ///     Insert::new("k1".into(), b"v1".to_vec()),
    ///     Insert::new("k2".into(), b"v2".to_vec()),
    /// ];
    /// db.insert_batch(&batch).unwrap();
    /// ```
    pub fn insert_batch(&mut self, msgs: &Vec<Insert>) -> Result<()> {
        self.db
            .transaction(|tx_db| {
                for msg in msgs {
                    tx_db.insert(msg.key().as_slice(), msg.value().to_vec())?;
                }
                Ok::<(), ConflictableTransactionError>(())
            })
            .context("Could not insert batch data into db")?;
        Ok(())
    }

    /// Removes the entry identified by the given message's key from the database.
    ///
    /// The provided `msg` supplies the key to delete; if the key exists it will be removed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut db = SledDb::new(&path, "default").unwrap();
    /// let remove_msg = Remove::new("my-key");
    /// db.remove(remove_msg).unwrap();
    /// ```
    pub fn remove(&mut self, msg: Remove) -> Result<()> {
        self.db
            .remove(msg.key())
            .context("Could not remove data from db")?;
        Ok(())
    }

    /// Fetches the value for the given key from the database.
    ///
    /// Returns `Some(Vec<u8>)` containing the value if the key exists, `None` if the key is not present.
    /// Any underlying I/O or database error is returned as an `Err` with context that includes the key.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::path::PathBuf;
    /// use crate::SledDb;
    /// use crate::Get;
    ///
    /// let path = PathBuf::from("/tmp/my_db");
    /// let mut db = SledDb::new(&path, "default").unwrap();
    /// // assume an Insert has been stored under key b"foo"
    /// let val = db.get(Get::new(b"foo".to_vec())).unwrap();
    /// if let Some(bytes) = val {
    ///     assert_eq!(bytes, b"expected value".to_vec());
    /// } else {
    ///     // key not found
    /// }
    /// ```
    pub fn get(&self, event: Get) -> Result<Option<Vec<u8>>> {
        let key = event.key();
        let str_key = String::from_utf8_lossy(&key).into_owned();
        let res = self
            .db
            .get(key)
            .context(format!("Failed to fetch {}", str_key))?;

        Ok(res.map(|v| v.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that SledDb instances share data when opened on the same path and remain isolated across different paths.
    ///
    /// This test exercises cross-instance visibility (writes from one instance are readable by another when using the same path/tree)
    /// and ensures separate paths do not share data.
    ///
    /// # Examples
    ///
    /// ```
    /// use tempfile::tempdir;
    /// let temp_dir = tempdir().unwrap();
    /// let db_path = temp_dir.path().join("test_cache.db");
    ///
    /// let mut db1 = SledDb::new(&db_path, "datastore").unwrap();
    /// db1.insert(Insert::new(b"test_key".to_vec(), b"test_value".to_vec())).unwrap();
    ///
    /// let mut db2 = SledDb::new(&db_path, "datastore").unwrap();
    /// assert_eq!(db2.get(Get::new(b"test_key".to_vec())).unwrap().unwrap(), b"test_value".to_vec());
    /// ```
    #[test]
    fn test_sled_db_caching() -> Result<()> {
        use tempfile::tempdir;

        // Section 1: Test basic cache functionality
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let db_path = temp_dir.path().join("test_cache.db");

        // Create first instance and insert data
        let mut db1 = SledDb::new(&db_path, "datastore")?;
        db1.insert(Insert::new(b"test_key".to_vec(), b"test_value".to_vec()))?;

        // Create second instance to same path and verify data access
        let mut db2 = SledDb::new(&db_path, "datastore")?;
        let result = db2.get(Get::new(b"test_key".to_vec()))?;
        assert_eq!(
            result.unwrap(),
            b"test_value".to_vec(),
            "Values from db2 should match"
        );

        // Cross-modify and verify (db1 writes, db2 reads)
        db1.insert(Insert::new(b"key2".to_vec(), b"value2".to_vec()))?;
        assert_eq!(
            db2.get(Get::new(b"key2".to_vec()))?.unwrap(),
            b"value2".to_vec(),
            "db2 should see changes from db1"
        );

        // Section 2: Test cross-instance operations (db2 writes, db1 reads)
        db2.insert(Insert::new(b"key3".to_vec(), b"value3".to_vec()))?;
        assert_eq!(
            db1.get(Get::new(b"key3".to_vec()))?.unwrap(),
            b"value3".to_vec(),
            "db1 should see changes from db2"
        );

        // Section 3: Test cache with different path
        let second_path = temp_dir.path().join("different_cache.db");
        let mut db3 = SledDb::new(&second_path, "datastore")?;
        db3.insert(Insert::new(b"db3_key".to_vec(), b"db3_value".to_vec()))?;

        // Create another instance to the second path
        let db4 = SledDb::new(&second_path, "datastore")?;
        assert_eq!(
            db4.get(Get::new(b"db3_key".to_vec()))?.unwrap(),
            b"db3_value".to_vec(),
            "db4 should see db3's data"
        );

        // Verify first path data isn't in second path
        assert!(
            db4.get(Get::new(b"test_key".to_vec()))?.is_none(),
            "db4 should not see data from db1/db2"
        );

        // Verify second path data isn't in first path
        assert!(
            db1.get(Get::new(b"db3_key".to_vec()))?.is_none(),
            "db1 should not see data from db3/db4"
        );

        Ok(())
    }

    #[test]
    fn test_sled_db_batch_insert() -> Result<()> {
        use tempfile::tempdir;

        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let db_path = temp_dir.path().join("test_batch.db");

        let mut db = SledDb::new(&db_path, "datastore")?;

        // Create a batch of inserts
        let batch = vec![
            Insert::new(b"batch_key1".to_vec(), b"batch_value1".to_vec()),
            Insert::new(b"batch_key2".to_vec(), b"batch_value2".to_vec()),
            Insert::new(b"batch_key3".to_vec(), b"batch_value3".to_vec()),
        ];

        // Insert the batch
        db.insert_batch(&batch)?;

        // Verify all items were inserted
        assert_eq!(
            db.get(Get::new(b"batch_key1".to_vec()))?.unwrap(),
            b"batch_value1".to_vec(),
            "First batch item should be retrievable"
        );
        assert_eq!(
            db.get(Get::new(b"batch_key2".to_vec()))?.unwrap(),
            b"batch_value2".to_vec(),
            "Second batch item should be retrievable"
        );
        assert_eq!(
            db.get(Get::new(b"batch_key3".to_vec()))?.unwrap(),
            b"batch_value3".to_vec(),
            "Third batch item should be retrievable"
        );

        // Verify non-existent key returns None
        assert!(
            db.get(Get::new(b"nonexistent".to_vec()))?.is_none(),
            "Non-existent key should return None"
        );

        Ok(())
    }
}