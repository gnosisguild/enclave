use crate::{InMemStore, IntoKey};
use actix::{Addr, Message, Recipient};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "()")]
pub struct Insert(pub Vec<u8>, pub Vec<u8>);
impl Insert {
    pub fn new<K: IntoKey>(key: K, value: Vec<u8>) -> Self {
        Self(key.into_key(), value)
    }

    pub fn key(&self) -> Vec<u8> {
        self.0.clone()
    }

    pub fn value(&self) -> Vec<u8> {
        self.1.clone()
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "Option<Vec<u8>>")]
pub struct Get(pub Vec<u8>);
impl Get {
    pub fn new<K: IntoKey>(key: K) -> Self {
        Self(key.into_key())
    }

    pub fn key(&self) -> Vec<u8> {
        self.0.clone()
    }
}

/// Generate proxy for the DB
#[derive(Clone)]
pub struct DataStore {
    scope: Vec<u8>,
    get: Recipient<Get>,
    insert: Recipient<Insert>,
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
            return;
        };
        let msg = Insert::new(&self.scope, serialized);
        self.insert.do_send(msg)
    }

    /// Construct a data store from an InMemStore actor
    pub fn from_in_mem(addr: &Addr<InMemStore>) -> Self {
        Self {
            get: addr.clone().recipient(),
            insert: addr.clone().recipient(),
            scope: vec![],
        }
    }

    /// Get the scope as a string
    pub fn get_scope(&self) -> Result<String> {
        Ok(String::from_utf8(self.scope.clone())?)
    }

    /// Changes the scope for the data store.
    /// Note that if the scope does not start with a slash one is appended.
    /// ```
    /// use data::DataStore;
    /// use data::InMemStore;
    /// use actix::Actor;
    /// use anyhow::Result;
    ///
    /// #[actix_rt::main]
    /// async fn main() -> Result<()>{  
    ///   let addr = InMemStore::new(false).start();
    ///   let store = DataStore::from_in_mem(&addr);
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
            scope,
        }
    }

    pub fn base<K: IntoKey>(&self, key: K) -> Self {
        Self {
            get: self.get.clone(),
            insert: self.insert.clone(),
            scope: key.into_key(),
        }
    }
}
