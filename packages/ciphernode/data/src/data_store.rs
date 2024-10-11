use actix::{Addr, Message, Recipient};
use anyhow::Result;

use crate::InMemDataStore;

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "()")]
pub struct Insert(pub Vec<u8>, pub Vec<u8>);
impl Insert {
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
    pub fn key(&self) -> Vec<u8> {
        self.0.clone()
    }
}

#[derive(Clone)]
pub struct DataStore(Recipient<Get>, Recipient<Insert>);
impl DataStore {
    pub async fn read(&self, msg: Get) -> Result<Option<Vec<u8>>> {
        Ok(self.0.send(msg).await?)
    }

    pub fn write(&self, msg: Insert) {
        self.1.do_send(msg)
    }

    // use this for testing
    pub fn from_in_mem(addr: Addr<InMemDataStore>) -> Self {
        Self(addr.clone().recipient(), addr.clone().recipient())
    }

    // // use this for production
    // pub fn from_sled(&data_addr: Addr<SledDb>) -> Self {
    //   let d = data_addr.clone();
    //   Self(d.recipient(),d.recipient())
    // }
}
