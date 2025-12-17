// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{Get, Insert, InsertBatch, InsertSync, Remove, SledDb};
use actix::{Actor, ActorContext, Addr, Handler};
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
    /// Creates and starts a SledStore actor backed by a SledDb at the given file system path.
    ///
    /// The actor is started immediately and subscribed to the bus "Shutdown" topic so it will
    /// drop its database and stop when a shutdown event is published.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assume `bus` and `path` are available in scope.
    /// let addr = SledStore::new(&bus, &path).unwrap();
    /// ```
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

    /// Handles an `Insert` message by writing the provided data to the backing database if available.
    ///
    /// If the store has an open database connection, attempts to insert the event into it.
    /// On insertion error, reports the error on the store's bus with `EType::Data`. Does nothing
    /// when the database is absent or the insertion succeeds.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Given `store: SledStore` and `ctx: <SledStore as Actor>::Context`
    /// // store.handle(Insert::new(key, value), &mut ctx);
    /// ```
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

    /// Handles an InsertBatch event by inserting its commands into the underlying database and reporting any insertion errors to the bus.
    ///
    /// The handler is a no-op when the database is not present.
    ///
    /// # Parameters
    ///
    /// - `event`: an `InsertBatch` containing the commands to insert; errors from the insertion are forwarded to the bus as `EType::Data`.
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
    /// Handles enclave lifecycle events for the actor.
    ///
    /// When the received `EnclaveEvent` carries `EnclaveEventData::Shutdown`, this handler drops
    /// the actor's database connection (if any) and stops the actor's context. For any other
    /// event data, no action is taken.
    ///
    /// # Examples
    ///
    /// ```
    /// // When an `EnclaveEvent` with `EnclaveEventData::Shutdown` is delivered to the actor,
    /// // the actor will drop its DB and stop:
    /// // let evt = EnclaveEvent::new(..., EnclaveEventData::Shutdown(...));
    /// // sled_store.handle(evt, &mut ctx);
    /// ```
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        if let EnclaveEventData::Shutdown(_) = msg.get_data() {
            let _db = self.db.take(); // db will be dropped
            ctx.stop()
        }
    }
}