// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{Get, Insert, InsertBatch, InsertSync, Remove, SledDb};
use actix::{Actor, ActorContext, Addr, AsyncContext, Handler};
use anyhow::Result;
use e3_events::{prelude::*, BusHandle, EType, EnclaveEvent, EnclaveEventData};
use std::path::PathBuf;
use tracing::{error, info};

pub struct SledStore {
    db: Option<SledDb>,
    bus: BusHandle, // Only used for Shutdown
}

impl Actor for SledStore {
    type Context = actix::Context<Self>;
}

impl SledStore {
    pub fn new(bus: &BusHandle, path: &PathBuf) -> Result<Addr<Self>> {
        info!("Starting SledStore with {:?}", path);
        let db = SledDb::new(path, "datastore")?;

        let store = Self {
            db: Some(db),
            bus: bus.clone(),
        }
        .start();

        bus.subscribe("Shutdown", store.clone().into());

        Ok(store)
    }
}

impl Handler<Insert> for SledStore {
    type Result = ();

    fn handle(&mut self, event: Insert, _: &mut Self::Context) -> Self::Result {
        if let Some(ref mut db) = &mut self.db {
            match db.insert(event) {
                Err(err) => self.bus.err(EType::Data, err),
                _ => (),
            }
        }
    }
}

impl Handler<InsertBatch> for SledStore {
    type Result = ();

    fn handle(&mut self, event: InsertBatch, ctx: &mut Self::Context) -> Self::Result {
        // XXX: handle this properly
        for cmd in event.commands() {
            ctx.notify(cmd.to_owned())
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
                Err(err) => self.bus.err(EType::Data, err),
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
                    self.bus.err(EType::Data, err);
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
        if let EnclaveEventData::Shutdown(_) = msg.get_data() {
            let _db = self.db.take(); // db will be dropped
            ctx.stop()
        }
    }
}
