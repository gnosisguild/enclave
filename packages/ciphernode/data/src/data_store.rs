use std::borrow::Cow;

use crate::{InMemStore, IntoKey, SledStore};
use actix::{Addr, Message, Recipient};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "()")]
pub struct Insert(pub Vec<u8>, pub Vec<u8>);
impl Insert {
    pub fn new<K: IntoKey>(key: K, value: Vec<u8>) -> Self {
        Self(key.into_key(), value)
    }

    pub fn key(&self) -> &Vec<u8> {
        &self.0
    }

    pub fn value(&self) -> &Vec<u8> {
        &self.1
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "Option<Vec<u8>>")]
pub struct Get(pub Vec<u8>);
impl Get {
    pub fn new<K: IntoKey>(key: K) -> Self {
        Self(key.into_key())
    }

    pub fn key(&self) -> &Vec<u8> {
        &self.0
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "()")]
pub struct Remove(pub Vec<u8>);
impl Remove {
    pub fn new<K: IntoKey>(key: K) -> Self {
        Self(key.into_key())
    }

    pub fn key(&self) -> &Vec<u8> {
        &self.0
    }
}

/// Generate proxy for the DB
#[derive(Clone, Debug)]
pub struct DataStore {
    scope: Vec<u8>,
    get: Recipient<Get>,
    insert: Recipient<Insert>,
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

    /// Removes data from the scope location
    pub fn clear(&self) {
        self.remove.do_send(Remove::new(&self.scope))
    }

    /// Get the scope as a string
    pub fn get_scope(&self) -> Result<Cow<str>> {
        Ok(String::from_utf8_lossy(&self.scope))
    }

    /// Changes the scope for the data store.
    /// Note that if the scope does not start with a slash one is appended.
    /// ```
    /// use data::DataStore;
    /// use data::InMemStore;
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
            get: self.get.clone(),
            insert: self.insert.clone(),
            remove: self.remove.clone(),
            scope,
        }
    }

    pub fn base<K: IntoKey>(&self, key: K) -> Self {
        Self {
            get: self.get.clone(),
            insert: self.insert.clone(),
            remove: self.remove.clone(),
            scope: key.into_key(),
        }
    }
}

impl From<&Addr<SledStore>> for DataStore {
    fn from(addr: &Addr<SledStore>) -> Self {
        Self {
            get: addr.clone().recipient(),
            insert: addr.clone().recipient(),
            remove: addr.clone().recipient(),
            scope: vec![],
        }
    }
}

impl From<&Addr<InMemStore>> for DataStore {
    fn from(addr: &Addr<InMemStore>) -> Self {
        Self {
            get: addr.clone().recipient(),
            insert: addr.clone().recipient(),
            remove: addr.clone().recipient(),
            scope: vec![],
        }
    }
}
