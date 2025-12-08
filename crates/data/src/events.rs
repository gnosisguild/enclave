// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::IntoKey;
use actix::Message;
use anyhow::Result;

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
#[rtype(result = "Result<()>")]
pub struct InsertSync(pub Vec<u8>, pub Vec<u8>);
impl InsertSync {
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

impl From<InsertSync> for Insert {
    fn from(value: InsertSync) -> Self {
        Insert::new(value.key(), value.value().clone())
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

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "()")]
pub struct SeekForPrev(pub Vec<u8>);
impl SeekForPrev {
    pub fn new<K: IntoKey>(key: K) -> Self {
        Self(key.into_key())
    }

    pub fn key(&self) -> &Vec<u8> {
        &self.0
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SeekForPrevResponse(u64);
impl SeekForPrevResponse {
    pub fn new(val: u64) -> Self {
        Self(val)
    }
}
