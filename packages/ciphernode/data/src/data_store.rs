use crate::InMemDataStore;
use actix::{Addr, Message, Recipient};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};


/// This trait allows our keys to be responsive to multiple inputs
pub trait IntoKey {
    fn into_key(self) -> Vec<u8>;
}

/// Keys can be vectors of String
impl IntoKey for Vec<String> {
    fn into_key(self) -> Vec<u8> {
        self.join("/").into_bytes()
    }
}

/// Keys can be vectors of &str
impl<'a> IntoKey for Vec<&'a str> {
    fn into_key(self) -> Vec<u8> {
        self.join("/").into_bytes()
    }
}

/// Keys can be String
impl IntoKey for String {
    fn into_key(self) -> Vec<u8> {
        self.into_bytes()
    }
}

/// Keys can be &String
impl IntoKey for &String {
    fn into_key(self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

/// Keys can be &str
impl<'a> IntoKey for &'a str {
    fn into_key(self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

/// Trait to add a prefix to a data storage object. This is used as a recursive trait for setting
/// scope on data objects which include the Get ad Insert commands for the data store
pub trait WithPrefix: Sized {
    fn prefix(&self, prefix: &str) -> Self;
    fn at(&self, key: &str) -> Self;
}


/// This trait enables the self type to report their state snapshot
pub trait Snapshot
where
    Self: Sized,
{
    /// The state must be serializable so that it can be stored as a value
    /// The Snapshot should represent all the dynamic data managed within the Actor or Object
    type Snapshot: Serialize + DeserializeOwned;

    /// Return a tuple with the first element being the id string of the object and the second
    /// being a representation of the object's state that is easily serialized by the data store
    fn snapshot(&self) -> Self::Snapshot;
}


/// This trait enables the self type to checkpoint its state
pub trait Checkpoint: Snapshot {
    /// Declare the DataStore instance available on the object
    fn get_store(&self) -> DataStore;

    /// Write the current snapshot to the DataStore provided by `get_store()` at the object's id returned by `get_id()`
    fn checkpoint(&self) {
        self.get_store().write(self.snapshot());
    }
}

/// Enable the self type to be reconstituted from the parameters coupled with the Snapshot
#[async_trait]
pub trait FromSnapshotWithParams: Snapshot {
    type Params: Send + 'static;

    /// Return an instance of the persistable object at the state given by the snapshot
    /// This method is async because there may be subobjects that require hydration from the store
    async fn from_snapshot(params: Self::Params, snapshot: Self::Snapshot) -> Result<Self>;
}

#[async_trait]
pub trait FromSnapshot: Snapshot {
    /// Return an instance of the persistable object at the state given by the snapshot
    /// This method is async because there may be subobjects that require hydration from the store
    async fn from_snapshot(snapshot: Self::Snapshot) -> Result<Self>;
}

impl WithPrefix for Vec<u8> {
    fn prefix(&self, prefix: &str) -> Self {
        let Ok(encoded) = String::from_utf8(self.clone()) else {
            // If this is not encoded as utf8 do nothing
            return self.clone();
        };
        vec![prefix.to_string(), encoded].join("/").into_bytes()
    }

    fn at(&self, key: &str) -> Self {
        key.to_string().into_bytes()
    }
}

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

impl WithPrefix for Insert {
    fn prefix(&self, prefix: &str) -> Self {
        Insert(self.0.prefix(prefix), self.1.clone())
    }

    fn at(&self, key: &str) -> Self {
        Insert(self.0.at(key), self.1.clone())
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

impl WithPrefix for Get {
    fn prefix(&self, prefix: &str) -> Self {
        Get(self.0.prefix(prefix))
    }
    fn at(&self, key: &str) -> Self {
        Get(self.0.at(key))
    }
}

#[derive(Clone)]
pub struct DataStore {
    prefix: Option<String>,
    get: Recipient<Get>,
    insert: Recipient<Insert>,
}

impl DataStore {
    pub async fn read<K, T>(&self, key: K) -> Result<Option<T>>
    where
        K: IntoKey,
        T: for<'de> Deserialize<'de>,
    {
        let msg = Get::new(key);
        let msg = self.prefix.as_ref().map_or(msg.clone(), |p| msg.prefix(p));
        let maybe_bytes = self.get.send(msg).await?;
        let Some(bytes) = maybe_bytes else {
            return Ok(None);
        };

        Ok(Some(bincode::deserialize(&bytes)?))
    }

    /// Writes anything serializable to the KV actor as a stream of bytes
    pub fn set<K: IntoKey, V: Serialize>(&self, key: K, value: V) {
        let Ok(serialized) = bincode::serialize(&value) else {
            return;
        };
        let msg = Insert::new(key, serialized);
        let msg = self.prefix.as_ref().map_or(msg.clone(), |p| msg.prefix(p));
        self.insert.do_send(msg)
    }

    /// Writes to whatever the prefix is set to on the datastore
    pub fn write<V: Serialize>(&self, value: V) {
        self.set("", value)
    }

    /// Read the value of the key starting at the root
    pub async fn read_at<K, T>(&self, key: K) -> Result<Option<T>>
    where
        K: IntoKey,
        T: for<'de> Deserialize<'de>,
    {
        self.at("").read(key).await
    }

    // use this for testing
    pub fn from_in_mem(addr: Addr<InMemDataStore>) -> Self {
        Self {
            get: addr.clone().recipient(),
            insert: addr.clone().recipient(),
            prefix: None,
        }
    }

    pub fn ensure_root_id(str: &str) -> Result<()> {
        if !str.starts_with("/") {
            return Err(anyhow!("string doesnt start with slash."));
        }
        Ok(())
    }

    // // use this for production
    // pub fn from_sled(&data_addr: Addr<SledDb>) -> Self {
    //   let d = data_addr.clone();
    //   Self(d.recipient(),d.recipient())
    // }
}

impl WithPrefix for DataStore {
    fn prefix(&self, prefix: &str) -> Self {
        Self {
            get: self.get.clone(),
            insert: self.insert.clone(),
            prefix: self.prefix.clone().map_or_else(
                || Some(prefix.to_string()),
                |p| Some(vec![prefix.to_string(), p].join("/")),
            ),
        }
    }

    fn at(&self, key: &str) -> Self {
        Self {
            get: self.get.clone(),
            insert: self.insert.clone(),
            prefix: Some(key.to_string()),
        }
    }
}
