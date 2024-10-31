use std::path::{Path, PathBuf};

use crate::{Get, Insert};
use actix::{Actor, Addr, Handler};
use anyhow::{Context, Result};
use enclave_core::{BusError, EnclaveErrorType, EventBus};
use sled::Db;

pub struct SledStore {
    db: SledDb,
    bus: Addr<EventBus>,
}

impl Actor for SledStore {
    type Context = actix::Context<Self>;
}

impl SledStore {
    pub fn new(bus: &Addr<EventBus>, path: &PathBuf) -> Result<Self> {
        let db = SledDb::new(path)?;
        Ok(Self {
            db,
            bus: bus.clone(),
        })
    }

    pub fn from_db(db: SledDb) -> Result<Self> {
        Ok(Self {
            db,
            bus: EventBus::new(false).start(),
        })
    }
}

impl Handler<Insert> for SledStore {
    type Result = ();

    fn handle(&mut self, event: Insert, _: &mut Self::Context) -> Self::Result {
        match self.db.insert(event) {
            Err(err) => self.bus.err(EnclaveErrorType::Data, err),
            _ => (),
        }
    }
}

impl Handler<Get> for SledStore {
    type Result = Option<Vec<u8>>;

    fn handle(&mut self, event: Get, _: &mut Self::Context) -> Self::Result {
        return match self.db.get(event) {
            Ok(v) => v,
            Err(err) => {
                self.bus.err(EnclaveErrorType::Data, err);
                None
            }
        };
    }
}

pub struct SledDb {
    db: Db,
}

impl SledDb {
    pub fn new(path: &PathBuf) -> Result<Self> {
        let db = sled::open(path).with_context(|| {
            format!(
                "Could not open database at path '{}'",
                path.to_string_lossy()
            )
        })?;
        Ok(Self { db })
    }

    pub fn insert(&mut self, msg: Insert) -> Result<()> {
        self.db
            .insert(msg.key(), msg.value().to_vec())
            .context("Could not insert data into db")?;

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
