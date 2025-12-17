// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{Get, Insert, InsertBatch, InsertSync, Remove};
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

    /// Construct an InMemStore from a bincode-serialized database.
    ///
    /// Attempts to deserialize `db` as a `BTreeMap<Vec<u8>, Vec<u8>>`. On success returns an `InMemStore` containing the deserialized map, the provided `capture` flag, and an empty operation log. Returns an error with context if deserialization fails.
    ///
    /// # Examples
    ///
    /// ```
    /// // Prepare a serialized empty DB for the example
    /// let bytes = bincode::serialize(&std::collections::BTreeMap::<Vec<u8>, Vec<u8>>::new()).unwrap();
    /// let store = from_dump(bytes, false).unwrap();
    /// // `store` is constructed successfully on successful deserialization
    /// let _ = store;
    /// ```
    pub fn from_dump(db: Vec<u8>, capture: bool) -> anyhow::Result<Self> {
        Ok(Self {
            db: bincode::deserialize(&db).context("Error deserializing BTreeMap")?,
            capture,
            log: vec![],
        })
    }
}

// Add a BatchInsert event that contains multiple Insert messages
// Use the Responder pattern to manage the response
// Have a proxy actor hold the Inserts until the BatchInsert event is called

impl Handler<Insert> for InMemStore {
    type Result = ();
    /// Stores the provided insert event's key and value in the in-memory database and, if capture is enabled, records the insert operation in the store's operation log.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crates::data::in_mem::{InMemStore, Insert, Get};
    /// # use actix::Context;
    /// let mut store = InMemStore::new(true);
    /// let msg = Insert::new(b"key".to_vec(), b"value".to_vec());
    /// // dispatch the insert handler (context value is not used by the handler)
    /// store.handle(msg.clone(), &mut Context::new());
    /// // the value is stored
    /// assert_eq!(store.db.get(b"key" as &[u8]), Some(&b"value".to_vec()));
    /// // the operation is recorded when capture is enabled
    /// assert!(matches!(store.log.last(), Some(crate::DataOp::Insert(_))));
    /// ```
    fn handle(&mut self, event: Insert, _: &mut Self::Context) {
        // insert data into sled
        self.db.insert(event.key().to_vec(), event.value().to_vec());

        if self.capture {
            self.log.push(DataOp::Insert(event));
        }
    }
}

impl Handler<InsertBatch> for InMemStore {
    type Result = ();
    /// Handles a batch insert message by inserting each command's key/value into the in-memory store
    /// and appending insert operations to the store's log when capture is enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use crates::data::in_mem::{InMemStore, Insert, InsertBatch, DataOp};
    ///
    /// // prepare store and batch
    /// let mut store = InMemStore::new(true);
    /// let cmd = Insert::new(b"key".to_vec(), b"val".to_vec());
    /// let batch = InsertBatch::from_vec(vec![cmd.clone()]);
    ///
    /// // emulate handler behavior: insert batch commands into the store
    /// for cmd in batch.commands() {
    ///     store.db.insert(cmd.key().to_owned(), cmd.value().to_owned());
    ///     if store.capture {
    ///         store.log.push(DataOp::Insert(cmd.clone()));
    ///     }
    /// }
    ///
    /// assert_eq!(store.db.get(b"key".as_ref()).map(|v| v.as_slice()), Some(b"val".as_ref()));
    /// assert_eq!(store.log.len(), 1);
    /// ```
    fn handle(&mut self, msg: InsertBatch, _: &mut Self::Context) -> Self::Result {
        for cmd in msg.commands() {
            self.db.insert(cmd.key().to_owned(), cmd.value().to_owned());
            if self.capture {
                self.log.push(DataOp::Insert(cmd.clone()));
            }
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
    /// Retrieves the value associated with the `Get` message's key from the in-memory database.
    ///
    /// # Returns
    ///
    /// `Some(Vec<u8>)` containing the stored value if the key exists, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// let mut db: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
    /// db.insert(b"key".to_vec(), b"value".to_vec());
    /// let result = db.get(&b"key".to_vec()).cloned();
    /// assert_eq!(result, Some(b"value".to_vec()));
    /// ```
    fn handle(&mut self, event: Get, _: &mut Self::Context) -> Option<Vec<u8>> {
        let key = event.key();
        let r = self.db.get(key);
        r.cloned()
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