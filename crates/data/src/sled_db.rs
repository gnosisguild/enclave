// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use sled::{Db, Tree};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tracing::info;

use crate::{
    traits::{KeyValStore, SeekableStore},
    Get, Insert, Remove, SeekForPrev,
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
}

impl KeyValStore for SledDb {
    fn insert(&mut self, msg: Insert) -> Result<()> {
        self.db
            .insert(msg.key(), msg.value().to_vec())
            .context("Could not insert data into db")?;

        Ok(())
    }

    fn remove(&mut self, msg: Remove) -> Result<()> {
        self.db
            .remove(msg.key())
            .context("Could not remove data from db")?;
        Ok(())
    }

    fn get(&self, event: Get) -> Result<Option<Vec<u8>>> {
        let key = event.key();
        let str_key = String::from_utf8_lossy(&key).into_owned();
        let res = self
            .db
            .get(key)
            .context(format!("Failed to fetch {}", str_key))?;

        Ok(res.map(|v| v.to_vec()))
    }
}

impl SeekableStore for SledDb {
    fn seek_for_prev(&self, msg: SeekForPrev) -> Result<Option<Vec<u8>>> {
        let key = msg.key();
        let entry = self.db.range(..=&key[..]).next_back();

        match entry {
            Some(Ok((_, bytes))) => Ok(Some(bytes.as_ref().try_into()?)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }
}

// Global static cache
pub static SLED_CACHE: Lazy<Arc<Mutex<HashMap<String, Db>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Returns a stable canonical string path used as a cache key.
// Canonicalizes the parent directory if the target path does not yet exist.
fn canonical_key(path: &PathBuf) -> String {
    use std::path::{Path, PathBuf};
    if path.exists() {
        return path
            .canonicalize()
            .unwrap_or_else(|_| path.clone())
            .to_string_lossy()
            .into_owned();
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let base: PathBuf = parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf());
    let tail = path.file_name().map(|s| s.to_owned()).unwrap_or_default();
    base.join(tail).to_string_lossy().into_owned()
}

// Opens or retrieves a cached sled database for the given path.
// Prevents conflicts by ensuring only a single connection was open to a db file at once per process.
// Ensures the directory exists and stabilizes the canonical key across OSes.
fn get_or_open_db(path: &PathBuf) -> Result<Db> {
    let _ = std::fs::create_dir_all(path);
    let key = canonical_key(path);
    let mut cache = SLED_CACHE.lock().unwrap();
    if let Some(db) = cache.get(&key) {
        return Ok(db.clone());
    }
    let db = sled::open(path).with_context(|| {
        format!(
            "Could not open database at path '{}'",
            path.to_string_lossy()
        )
    })?;
    cache.insert(key, db.clone());
    if !db.was_recovered() {
        info!("created db at: {:?}", &path);
    } else {
        info!("recovered db st: {:?}", &path);
    }

    Ok(db)
}

fn get_or_open_db_tree(path: &PathBuf, tree: &str) -> Result<Tree> {
    let db = get_or_open_db(path)?;
    Ok(db.open_tree(tree)?)
}

fn clear_all_caches() {
    let mut cache_lock = SLED_CACHE.lock().unwrap();
    cache_lock.clear();
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
