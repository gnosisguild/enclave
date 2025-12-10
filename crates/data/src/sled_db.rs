// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{Context, Result};
use sled::Tree;
use std::path::PathBuf;

use crate::{
    sled_utils::{clear_all_caches, get_or_open_db_tree},
    Get, Insert, Remove,
};

pub struct SledDb {
    db: Tree,
}

impl SledDb {
    pub fn new(path: &PathBuf, tree: &str) -> Result<Self> {
        let db = get_or_open_db_tree(path, tree)?;
        Ok(Self { db })
    }

    pub fn close_all_connections() {
        clear_all_caches()
    }

    pub fn insert(&mut self, msg: Insert) -> Result<()> {
        self.db
            .insert(msg.key(), msg.value().to_vec())
            .context("Could not insert data into db")?;

        Ok(())
    }

    pub fn remove(&mut self, msg: Remove) -> Result<()> {
        self.db
            .remove(msg.key())
            .context("Could not remove data from db")?;
        Ok(())
    }

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
}
