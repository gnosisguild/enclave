use std::path::PathBuf;

use crate::{Get, Insert, Remove};
use actix::{Actor, ActorContext, Addr, Handler};
use anyhow::{Context, Result};
use events::{BusError, EnclaveErrorType, EnclaveEvent, EventBus, EventBusConfig, Subscribe};
use sled::Db;
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
        let db = SledDb::new(path)?;

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
            bus: EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start(),
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
