// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::SledDb;
use actix::{Actor, ActorContext, Addr, Handler};
use anyhow::Result;
use e3_events::{
    prelude::*, BusHandle, EType, EnclaveEvent, EnclaveEventData,
    EnclaveUnsequencedErrorDispatcher, EventType,
};
use e3_events::{Get, Insert, InsertBatch, InsertSync, Remove};
use e3_utils::MAILBOX_LIMIT;
use std::path::PathBuf;
use tracing::{error, info};

pub struct SledStore {
    db: Option<SledDb>,
    bus: Box<dyn EnclaveUnsequencedErrorDispatcher>, // Only used for Shutdown
}

impl Actor for SledStore {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT)
    }
}

impl SledStore {
    pub fn new<S: 'static>(bus: &BusHandle<S>, path: &PathBuf) -> Result<Addr<Self>> {
        // Note we pass in a generic BusHandle which supports the err method for passing on errors.
        // This was as stores are required before we can initialize the BusHandle to retrieve the
        // address so we have a unique node_id.
        // If BusHandle is Disabled that is fine as our subscriptions and error publishing function
        // remains intact despite it being enabled elsewhere at a later point
        info!("Starting SledStore with {:?}", path);
        let db = SledDb::new(path, "datastore")?;

        let store = Self {
            db: Some(db),
            bus: Box::new(bus.clone()),
        }
        .start();

        bus.subscribe(EventType::Shutdown, store.clone().into());

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

    fn handle(&mut self, event: InsertBatch, _: &mut Self::Context) -> Self::Result {
        if let Some(ref mut db) = &mut self.db {
            match db.insert_batch(event.commands()) {
                Err(err) => self.bus.err(EType::Data, err),
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
