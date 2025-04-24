use crate::{Get, Insert, InsertSync, Remove};
use actix::{Actor, ActorContext, Addr, Handler};
use anyhow::{Context, Result};
use events::{BusError, EnclaveErrorType, EnclaveEvent, EventBus, EventBusConfig, Subscribe};
use once_cell::sync::Lazy;
use sled::Db;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tracing::{error, info};

pub struct SledStore {
    db: Option<SledDb>,
    bus: Addr<EventBus<EnclaveEvent>>,
}

impl Actor for SledStore {
    type Context = actix::Context<Self>;
}

impl SledStore {
    pub fn new(bus: &Addr<EventBus<EnclaveEvent>>, path: &PathBuf) -> Result<Addr<Self>> {
        info!("Starting SledStore");
        let db = SledDb::new(PathBuf::from(path))?;

        let store = Self {
            db: Some(db),
            bus: bus.clone(),
        }
        .start();

        bus.do_send(Subscribe::new("Shutdown", store.clone().into()));

        Ok(store)
    }

    pub fn from_db(db: SledDb) -> Result<Self> {
        Ok(Self {
            db: Some(db),
            bus: EventBus::<EnclaveEvent>::new(EventBusConfig {
                capture_history: false,
                deduplicate: true,
            })
            .start(),
        })
    }
}

impl Handler<Insert> for SledStore {
    type Result = ();

    fn handle(&mut self, event: Insert, _: &mut Self::Context) -> Self::Result {
        if let Some(ref mut db) = &mut self.db {
            match db.insert(event) {
                Err(err) => self.bus.err(EnclaveErrorType::Data, err),
                _ => (),
            }
        }
    }
}

impl Handler<InsertSync> for SledStore {
    type Result = Result<()>;

    fn handle(&mut self, event: InsertSync, _: &mut Self::Context) -> Self::Result {
        if let Some(ref mut db) = &mut self.db {
            db.insert(event.into())
                .map_err(|e| anyhow::anyhow!("{}", e.to_string()))?
        }
        Ok(())
    }
}

impl Handler<Remove> for SledStore {
    type Result = ();

    fn handle(&mut self, event: Remove, _: &mut Self::Context) -> Self::Result {
        if let Some(ref mut db) = &mut self.db {
            match db.remove(event) {
                Err(err) => self.bus.err(EnclaveErrorType::Data, err),
                _ => (),
            }
        }
    }
}

impl Handler<Get> for SledStore {
    type Result = Option<Vec<u8>>;

    fn handle(&mut self, event: Get, _: &mut Self::Context) -> Self::Result {
        if let Some(ref mut db) = &mut self.db {
            return match db.get(event) {
                Ok(v) => v,
                Err(err) => {
                    self.bus.err(EnclaveErrorType::Data, err);
                    None
                }
            };
        } else {
            error!("Attempt to get data from dropped db");
            None
        }
    }
}

impl Handler<EnclaveEvent> for SledStore {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        if let EnclaveEvent::Shutdown { .. } = msg {
            let _db = self.db.take(); // db will be dropped
            ctx.stop()
        }
    }
}

pub struct SledDb {
    db: Db,
}

// Global static cache
pub static SLED_CACHE: Lazy<Arc<Mutex<HashMap<String, Db>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

fn get_cached_db(path: &PathBuf) -> Option<Db> {
    let key = path.to_string_lossy().to_string();
    let cache_lock = SLED_CACHE.lock().unwrap();
    let maybe_db = cache_lock.get(&key).cloned();
    maybe_db
}

fn set_cached_db(path: PathBuf, db: Db) {
    let key = path.to_string_lossy().to_string();
    let mut cache_lock = SLED_CACHE.lock().unwrap();
    cache_lock.insert(key, db);
}

impl SledDb {
    pub fn new(path: PathBuf) -> Result<Self> {
        let maybe_db = get_cached_db(&path);
        if let Some(db) = maybe_db {
            return Ok(Self { db });
        };

        let db = sled::open(&path).with_context(|| {
            format!(
                "Could not open database at path '{}'",
                path.to_string_lossy()
            )
        })?;

        set_cached_db(path, db.clone());
        Ok(Self { db })
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

    pub fn get(&mut self, event: Get) -> Result<Option<Vec<u8>>> {
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
        let mut db1 = SledDb::new(db_path.clone())?;
        db1.insert(Insert::new(b"test_key".to_vec(), b"test_value".to_vec()))?;

        // Create second instance to same path and verify data access
        let mut db2 = SledDb::new(db_path.clone())?;
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
        let mut db3 = SledDb::new(second_path.clone())?;
        db3.insert(Insert::new(b"db3_key".to_vec(), b"db3_value".to_vec()))?;

        // Create another instance to the second path
        let mut db4 = SledDb::new(second_path)?;
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
