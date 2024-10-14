use actix::{Addr, Message, Recipient};
use anyhow::{anyhow, Result};

use crate::InMemDataStore;

pub trait IntoKey {
    fn into_key(self) -> Vec<u8>;
}

impl IntoKey for Vec<String> {
    fn into_key(self) -> Vec<u8> {
        self.join("/").into_bytes()
    }
}

impl<'a> IntoKey for Vec<&'a str> {
    fn into_key(self) -> Vec<u8> {
        self.join("/").into_bytes()
    }
}

impl IntoKey for String {
    fn into_key(self) -> Vec<u8> {
        self.into_bytes()
    }
}

impl IntoKey for &String {
    fn into_key(self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

impl<'a> IntoKey for &'a str {
    fn into_key(self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

pub trait WithPrefix: Sized {
    fn prefix(self, prefix: &str) -> Self;
    fn base(self, key: &str) -> Self;
}

impl WithPrefix for Vec<u8> {
    fn prefix(self, prefix: &str) -> Self {
        let Ok(encoded) = String::from_utf8(self.clone()) else {
            // If this is not encoded as utf8 do nothing
            return self;
        };
        vec![prefix.to_string(), encoded].join("/").into_bytes()
    }

    fn base(self, key: &str) -> Self {
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
    fn prefix(self, prefix: &str) -> Self {
        Insert(self.0.prefix(prefix), self.1)
    }

    fn base(self, key: &str) -> Self {
        Insert(self.0.base(key), self.1)
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
    fn prefix(self, prefix: &str) -> Self {
        Get(self.0.prefix(prefix))
    }
    fn base(self, key: &str) -> Self {
        Get(self.0.base(key))
    }
}

#[derive(Clone)]
pub struct DataStore {
    prefix: Option<String>,
    get: Recipient<Get>,
    insert: Recipient<Insert>,
}

impl DataStore {
    pub async fn read<K:IntoKey>(&self, key: K) -> Result<Option<Vec<u8>>> {
        let msg = Get::new(key);
        let msg = self.prefix.as_ref().map_or(msg.clone(), |p| msg.prefix(p));
        Ok(self.get.send(msg).await?)
    }

    pub fn write<K:IntoKey>(&self, key: K, value: Vec<u8>) {
        let msg = Insert::new(key, value);
        let msg = self.prefix.as_ref().map_or(msg.clone(), |p| msg.prefix(p));
        self.insert.do_send(msg)
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
    fn prefix(self, prefix: &str) -> Self {
        Self {
            get: self.get,
            insert: self.insert,
            prefix: self.prefix.map_or_else(
                || Some(prefix.to_string()),
                |p| Some(vec![prefix.to_string(), p].join("/")),
            ),
        }
    }

    fn base(self, key: &str) -> Self {
        Self {
            get: self.get,
            insert: self.insert,
            prefix: Some(key.to_string()),
        }
    }
}
