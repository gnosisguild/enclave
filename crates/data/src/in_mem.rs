// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{Get, Insert, InsertSync, Remove};
use actix::{Actor, Handler, Message};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "Vec<DataOp>")]
pub struct GetLog;

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "anyhow::Result<Vec<u8>>")]
pub struct GetDump;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DataOp {
    Insert(Insert),
    Remove(Remove),
}

pub struct InMemStore {
    db: BTreeMap<Vec<u8>, Vec<u8>>,
    log: Vec<DataOp>,
    capture: bool,
}

impl Actor for InMemStore {
    type Context = actix::Context<Self>;
}

impl InMemStore {
    pub fn new(capture: bool) -> Self {
        Self {
            db: BTreeMap::new(),
            capture,
            log: vec![],
        }
    }

    pub fn get_dump(&self) -> Result<Vec<u8>> {
        bincode::serialize(&self.db.clone()).context("Error serializing BTreeMap")
    }

    pub fn from_dump(db: Vec<u8>, capture: bool) -> anyhow::Result<Self> {
        Ok(Self {
            db: bincode::deserialize(&db).context("Error deserializing BTreeMap")?,
            capture,
            log: vec![],
        })
    }
}

impl Handler<Insert> for InMemStore {
    type Result = ();
    fn handle(&mut self, event: Insert, _: &mut Self::Context) {
        // insert data into sled
        self.db.insert(event.key().to_vec(), event.value().to_vec());

        if self.capture {
            self.log.push(DataOp::Insert(event));
        }
    }
}

impl Handler<InsertSync> for InMemStore {
    type Result = Result<()>;

    fn handle(&mut self, event: InsertSync, _: &mut Self::Context) -> Self::Result {
        self.db.insert(event.key().to_vec(), event.value().to_vec());
        if self.capture {
            self.log.push(DataOp::Insert(event.into()));
        }
        Ok(())
    }
}

impl Handler<Remove> for InMemStore {
    type Result = ();
    fn handle(&mut self, event: Remove, _: &mut Self::Context) {
        // insert data into sled
        self.db.remove(&event.key().to_vec());

        if self.capture {
            self.log.push(DataOp::Remove(event));
        }
    }
}

impl Handler<Get> for InMemStore {
    type Result = Option<Vec<u8>>;
    fn handle(&mut self, event: Get, _: &mut Self::Context) -> Option<Vec<u8>> {
        let key = event.key();
        self.db.get(key).cloned()
    }
}

impl Handler<GetLog> for InMemStore {
    type Result = Vec<DataOp>;
    fn handle(&mut self, _: GetLog, _: &mut Self::Context) -> Vec<DataOp> {
        self.log.clone()
    }
}

impl Handler<GetDump> for InMemStore {
    type Result = anyhow::Result<Vec<u8>>;
    fn handle(&mut self, _: GetDump, _: &mut Self::Context) -> Self::Result {
        self.get_dump()
    }
}
