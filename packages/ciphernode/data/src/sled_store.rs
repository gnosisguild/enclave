use crate::{Get, Insert};
use actix::{Actor, Addr, Handler};
use anyhow::{Context, Result};
use enclave_core::{BusError, EnclaveErrorType, EventBus};
use sled::{Db, IVec};

pub struct SledStore {
    db: Db,
    bus: Addr<EventBus>,
}

impl Actor for SledStore {
    type Context = actix::Context<Self>;
}

impl SledStore {
    pub fn new(bus: &Addr<EventBus>, path: &str) -> Result<Self> {
        let db = sled::open(path).context("could not open db")?;
        Ok(Self {
            db,
            bus: bus.clone(),
        })
    }
}

impl Handler<Insert> for SledStore {
    type Result = ();

    fn handle(&mut self, event: Insert, _: &mut Self::Context) -> Self::Result {
        match self
            .db
            .insert(event.key(), event.value())
            .context("Could not insert data into db")
        {
            Err(err) => self.bus.err(EnclaveErrorType::Data, err),
            _ => (),
        }
    }
}

impl Handler<Get> for SledStore {
    type Result = Option<Vec<u8>>;

    fn handle(&mut self, event: Get, _: &mut Self::Context) -> Self::Result {
        let key = event.key();
        let str_key = String::from_utf8_lossy(&key).into_owned();
        let res: Result<Option<IVec>> = self
            .db
            .get(key)
            .context(format!("Failed to fetch {}", str_key));

        return match res {
            Ok(value) => value.map(|v| v.to_vec()),
            Err(err) => {
                self.bus.err(EnclaveErrorType::Data, err);
                None
            }
        };
    }
}
