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
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tracing::info;

// Global static cache
pub static SLED_CACHE: Lazy<Arc<Mutex<HashMap<String, Db>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Returns a stable canonical string path used as a cache key.
// Canonicalizes the parent directory if the target path does not yet exist.
/// Produce a stable string key for a filesystem path suitable for cache indexing.
///
/// If the given path exists, its canonical form is used. If the path does not exist,
/// the parent directory is canonicalized (or "." if no parent) and the path's final
/// component (file name) is appended to that canonical parent to form the key. The
/// result is returned as an owned `String`.
///
/// # Parameters
///
/// - `path`: the filesystem path to convert into a stable key.
///
/// # Returns
///
/// An owned string representing a stable, canonicalized key for `path`.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// let p = PathBuf::from("some/nonexistent/path.txt");
/// let key = canonical_key(&p);
/// assert!(key.ends_with("path.txt"));
/// ```
fn canonical_key(path: &PathBuf) -> String {
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
/// Opens a sled database at the given path or returns a cached handle for that path.
///
/// Ensures the target directory exists, reuses a previously opened database for the same
/// canonical path when available, and logs whether the database was created or recovered.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// // let db = get_or_open_db(&PathBuf::from("my_db")).unwrap();
/// ```
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

/// Open or create the named sled `Tree` in the database at `path`.
///
/// The returned tree is created if it does not already exist.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
///
/// let path = PathBuf::from("my_db");
/// let tree = get_or_open_db_tree(&path, "values").unwrap();
/// // use `tree`...
/// ```
///
/// # Returns
///
/// The `Tree` for the specified name.
pub fn get_or_open_db_tree(path: &PathBuf, tree: &str) -> Result<Tree> {
    let db = get_or_open_db(path)?;
    Ok(db.open_tree(tree)?)
}

/// Clears the process-global in-memory cache of opened sled databases.
///
/// This removes all entries from the global cache, dropping the stored `sled::Db` handles.
///
/// # Examples
///
/// ```
/// use std::sync::MutexGuard;
/// // Call to clear the cache
/// clear_all_caches();
/// // Verify the cache is empty
/// let guard = SLED_CACHE.lock().unwrap();
/// assert!(guard.is_empty());
/// ```
pub fn clear_all_caches() {
    let mut cache_lock = SLED_CACHE.lock().unwrap();
    cache_lock.clear();
}