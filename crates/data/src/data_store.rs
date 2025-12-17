// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::borrow::Cow;

use crate::{Get, Insert, InsertSync, Remove, WriteBuffer};
use crate::{InMemStore, IntoKey, SledStore};
use actix::{Addr, Recipient};
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Clone, Debug)]
pub enum StoreAddr {
    InMem(Addr<InMemStore>),
    Sled(Addr<SledStore>),
}

impl StoreAddr {
    /// Get the `InMemStore` actor address when this `StoreAddr` is the `InMem` variant.
    ///
    /// # Returns
    ///
    /// `Some(&Addr<InMemStore>)` if the variant is `InMem`, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crate::StoreAddr;
    /// let store_addr = /* obtain a StoreAddr */ unimplemented!();
    /// if let Some(in_mem_addr) = store_addr.to_maybe_in_mem() {
    ///     // use the in-memory store actor address
    /// }
    /// ```
    pub fn to_maybe_in_mem(&self) -> Option<&Addr<InMemStore>> {
        match self {
            StoreAddr::InMem(ref store) => Some(store),
            _ => None,
        }
    }
}

/// Generate proxy for the DB / KV store
/// DataStore is scopable
#[derive(Clone, Debug)]
pub struct DataStore {
    scope: Vec<u8>,
    addr: StoreAddr,
    get: Recipient<Get>,
    insert: Recipient<Insert>,
    insert_sync: Recipient<InsertSync>,
    remove: Recipient<Remove>,
}

impl DataStore {
    /// Read data at the scope location
    pub async fn read<T>(&self) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let Some(bytes) = self.get.send(Get::new(&self.scope)).await? else {
            return Ok(None);
        };

        // If we get a null value return None as this doesn't deserialize correctly
        if bytes == [0] {
            return Ok(None);
        }

        Ok(Some(bincode::deserialize(&bytes)?))
    }

    /// Writes data to the scope location
    pub fn write<T: Serialize>(&self, value: T) {
        let Ok(serialized) = bincode::serialize(&value) else {
            let str_key = self.get_scope().unwrap_or(Cow::Borrowed("<bad key>"));
            let str_error = format!("Could not serialize value passed to {}", str_key);
            error!(str_error);
            return;
        };
        let msg = Insert::new(&self.scope, serialized);
        self.insert.do_send(msg)
    }

    /// Writes data syncronously to the scope location
    pub async fn write_sync<T: Serialize>(&self, value: T) -> Result<()> {
        let serialized = bincode::serialize(&value).with_context(|| {
            let str_key = self.get_scope().unwrap_or(Cow::Borrowed("<bad key>"));
            anyhow!("Could not serialize value passed to {}", str_key)
        })?;

        let msg = InsertSync::new(&self.scope, serialized);
        self.insert_sync.send(msg).await??;
        Ok(())
    }

    /// Removes data from the scope location
    pub fn clear(&self) {
        self.remove.do_send(Remove::new(&self.scope))
    }

    /// Get the scope as a string
    pub fn get_scope(&self) -> Result<Cow<str>> {
        Ok(String::from_utf8_lossy(&self.scope))
    }

    /// Get a reference to the addr enum
    pub fn get_addr(&self) -> &StoreAddr {
        &self.addr
    }

    /// Changes the scope for the data store.
    /// Note that if the scope does not start with a slash one is appended.
    /// ```
    /// use e3_data::DataStore;
    /// use e3_data::InMemStore;
    /// use actix::Actor;
    /// use anyhow::Result;
    ///
    /// #[actix::main]
    /// async fn main() -> Result<()>{  
    ///   let addr = InMemStore::new(false).start();
    ///   let store = DataStore::from(&addr);
    ///   assert_eq!(store.base("//foo")
    ///     .scope("bar")
    ///     .scope("/baz")
    ///     .get_scope()?, "//foo/bar/baz");
    ///   Ok(())
    /// }
    /// ```
    pub fn scope<K: IntoKey>(&self, key: K) -> Self {
        let mut scope = self.scope.clone();
        let encoded_key = key.into_key();
        if !encoded_key.starts_with(&[b'/']) {
            scope.extend("/".into_key());
        }
        scope.extend(encoded_key);
        Self {
            addr: self.addr.clone(),
            get: self.get.clone(),
            insert: self.insert.clone(),
            insert_sync: self.insert_sync.clone(),
            remove: self.remove.clone(),
            scope,
        }
    }

    /// Create a new DataStore whose scope is set to the given key while preserving the same store address and actor recipients.
    ///
    /// The returned DataStore shares the original instance's address and message recipients but uses `key` as its absolute scope (replacing the current scope).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Given an existing `DataStore` instance `ds`, create a new root store at "settings".
    /// let child = ds.base("settings");
    /// assert_eq!(child.get_scope().unwrap(), "settings");
    /// ```
    pub fn base<K: IntoKey>(&self, key: K) -> Self {
        Self {
            addr: self.addr.clone(),
            get: self.get.clone(),
            insert: self.insert.clone(),
            insert_sync: self.insert_sync.clone(),
            remove: self.remove.clone(),
            scope: key.into_key(),
        }
    }

    /// Constructs a DataStore backed by the provided Sled store and wired to the given WriteBuffer.
    ///
    /// The resulting DataStore uses `addr` for reads, synchronous writes, and removals, and uses
    /// `write_buffer` for buffered insert operations. The returned store has an empty scope.
    ///
    /// # Parameters
    /// - `addr`: address of the SledStore actor.
    /// - `write_buffer`: address of the WriteBuffer actor.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given existing actix addresses `sled_addr: Addr<SledStore>` and `wb_addr: Addr<WriteBuffer>`:
    /// let ds = DataStore::from_sled_store(&sled_addr, &wb_addr);
    /// // new DataStore starts with an empty scope
    /// assert_eq!(ds.get_scope().unwrap(), "");
    /// ```
    pub fn from_sled_store(addr: &Addr<SledStore>, write_buffer: &Addr<WriteBuffer>) -> Self {
        Self {
            addr: StoreAddr::Sled(addr.clone()),
            get: addr.clone().recipient(),
            insert: write_buffer.clone().recipient(),
            insert_sync: addr.clone().recipient(),
            remove: addr.clone().recipient(),
            scope: vec![],
        }
    }

    /// Creates a DataStore configured to use the provided in-memory store and write buffer.
    ///
    /// `addr` is the actor address of the InMemStore used for reads, synchronous inserts, and removals.
    /// `write_buffer` is the actor address used for buffered (asynchronous) inserts.
    ///
    /// Returns a DataStore that uses the given in-memory store for direct operations and the write buffer
    /// for buffered insertions. The returned DataStore has an empty initial scope.
    ///
    /// # Examples
    ///
    /// ```
    /// // assume `in_mem_addr` and `write_buffer_addr` are available Addr<...> values
    /// let ds = DataStore::from_in_mem(&in_mem_addr, &write_buffer_addr);
    /// ```
    pub fn from_in_mem(addr: &Addr<InMemStore>, write_buffer: &Addr<WriteBuffer>) -> Self {
        Self {
            addr: StoreAddr::InMem(addr.clone()),
            get: addr.clone().recipient(),
            insert: write_buffer.clone().recipient(),
            insert_sync: addr.clone().recipient(),
            remove: addr.clone().recipient(),
            scope: vec![],
        }
    }
}

impl From<&Addr<SledStore>> for DataStore {
    fn from(addr: &Addr<SledStore>) -> Self {
        Self {
            addr: StoreAddr::Sled(addr.clone()),
            get: addr.clone().recipient(),
            insert: addr.clone().recipient(),
            insert_sync: addr.clone().recipient(),
            remove: addr.clone().recipient(),
            scope: vec![],
        }
    }
}

impl From<&Addr<InMemStore>> for DataStore {
    fn from(addr: &Addr<InMemStore>) -> Self {
        Self {
            addr: StoreAddr::InMem(addr.clone()),
            get: addr.clone().recipient(),
            insert: addr.clone().recipient(),
            insert_sync: addr.clone().recipient(),
            remove: addr.clone().recipient(),
            scope: vec![],
        }
    }
}