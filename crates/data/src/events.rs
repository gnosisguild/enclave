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
    /// Creates a new message containing the provided key and value, converting the key to a `Vec<u8>` via `IntoKey`.
    ///
    /// # Examples
    ///
    /// ```
    /// let msg = Insert::new("my-key", vec![1, 2, 3]);
    /// assert_eq!(msg.key(), &"my-key".into_key());
    /// assert_eq!(msg.value(), &vec![1, 2, 3]);
    /// ```
    pub fn new<K: IntoKey>(key: K, value: Vec<u8>) -> Self {
        Self(key.into_key(), value)
    }

    /// Get a reference to the key bytes stored in the message.
    ///
    /// # Examples
    ///
    /// ```
    /// // construct an Insert message and borrow its key
    /// let msg = crate::events::Insert(Vec::from(b"key".as_slice()), Vec::from(b"val".as_slice()));
    /// assert_eq!(msg.key(), &Vec::from(b"key".as_slice()));
    /// ```
    pub fn key(&self) -> &Vec<u8> {
        &self.0
    }

    /// Accesses the stored value bytes for this message.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::events::Insert;
    ///
    /// let msg = Insert(vec![b'k'], vec![1, 2, 3]);
    /// assert_eq!(msg.value(), &vec![1, 2, 3]);
    /// ```
    pub fn value(&self) -> &Vec<u8> {
        &self.1
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "()")]
pub struct InsertBatch(pub Vec<Insert>);
impl InsertBatch {
    /// Creates an `InsertBatch` containing the provided insert commands.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Insert::new("key", vec![1, 2, 3]);
    /// let batch = InsertBatch::new(vec![cmd.clone()]);
    /// assert_eq!(batch.commands().len(), 1);
    /// assert_eq!(batch.commands()[0], cmd);
    /// ```
    pub fn new(commands: Vec<Insert>) -> Self {
        Self(commands)
    }

    /// Accesses the batch's insert commands.
    ///
    /// Returns a reference to the vector of `Insert` commands contained in this batch.
    ///
    /// # Examples
    ///
    /// ```
    /// let cmd = Insert::new("key", vec![1, 2, 3]);
    /// let batch = InsertBatch::new(vec![cmd.clone()]);
    /// assert_eq!(batch.commands(), &vec![cmd]);
    /// ```
    pub fn commands(&self) -> &Vec<Insert> {
        &self.0
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "Result<()>")]
pub struct InsertSync(pub Vec<u8>, pub Vec<u8>);
impl InsertSync {
    /// Creates a new message containing the provided key and value, converting the key to a `Vec<u8>` via `IntoKey`.
    ///
    /// # Examples
    ///
    /// ```
    /// let msg = Insert::new("my-key", vec![1, 2, 3]);
    /// assert_eq!(msg.key(), &"my-key".into_key());
    /// assert_eq!(msg.value(), &vec![1, 2, 3]);
    /// ```
    pub fn new<K: IntoKey>(key: K, value: Vec<u8>) -> Self {
        Self(key.into_key(), value)
    }

    /// Get a reference to the key bytes stored in the message.
    ///
    /// # Examples
    ///
    /// ```
    /// // construct an Insert message and borrow its key
    /// let msg = crate::events::Insert(Vec::from(b"key".as_slice()), Vec::from(b"val".as_slice()));
    /// assert_eq!(msg.key(), &Vec::from(b"key".as_slice()));
    /// ```
    pub fn key(&self) -> &Vec<u8> {
        &self.0
    }

    /// Accesses the stored value bytes for this message.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::events::Insert;
    ///
    /// let msg = Insert(vec![b'k'], vec![1, 2, 3]);
    /// assert_eq!(msg.value(), &vec![1, 2, 3]);
    /// ```
    pub fn value(&self) -> &Vec<u8> {
        &self.1
    }
}

impl From<InsertSync> for Insert {
    /// Converts an `InsertSync` into an `Insert`, preserving the key and cloning the value.
    ///
    /// # Examples
    ///
    /// ```
    /// let sync = InsertSync::new("k", vec![1, 2, 3]);
    /// let insert: Insert = Insert::from(sync);
    /// assert_eq!(insert.key(), &b"k".to_vec());
    /// assert_eq!(insert.value(), &vec![1, 2, 3]);
    /// ```
    fn from(value: InsertSync) -> Self {
        Insert::new(value.key(), value.value().clone())
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "Option<Vec<u8>>")]
pub struct Get(pub Vec<u8>);
impl Get {
    /// Creates a new `Get` message from a key convertible via `IntoKey`.
    ///
    /// The provided `key` is converted into a `Vec<u8>` using `IntoKey` and stored inside the message.
    ///
    /// # Examples
    ///
    /// ```
    /// let msg = Get::new("my-key");
    /// assert_eq!(msg.key(), &b"my-key".to_vec());
    /// ```
    pub fn new<K: IntoKey>(key: K) -> Self {
        Self(key.into_key())
    }

    /// Get a reference to the key bytes stored in the message.
    ///
    /// # Examples
    ///
    /// ```
    /// // construct an Insert message and borrow its key
    /// let msg = crate::events::Insert(Vec::from(b"key".as_slice()), Vec::from(b"val".as_slice()));
    /// assert_eq!(msg.key(), &Vec::from(b"key".as_slice()));
    /// ```
    pub fn key(&self) -> &Vec<u8> {
        &self.0
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "()")]
pub struct Remove(pub Vec<u8>);
impl Remove {
    /// Creates a new `Get` message from a key convertible via `IntoKey`.
    ///
    /// The provided `key` is converted into a `Vec<u8>` using `IntoKey` and stored inside the message.
    ///
    /// # Examples
    ///
    /// ```
    /// let msg = Get::new("my-key");
    /// assert_eq!(msg.key(), &b"my-key".to_vec());
    /// ```
    pub fn new<K: IntoKey>(key: K) -> Self {
        Self(key.into_key())
    }

    /// Get a reference to the key bytes stored in the message.
    ///
    /// # Examples
    ///
    /// ```
    /// // construct an Insert message and borrow its key
    /// let msg = crate::events::Insert(Vec::from(b"key".as_slice()), Vec::from(b"val".as_slice()));
    /// assert_eq!(msg.key(), &Vec::from(b"key".as_slice()));
    /// ```
    pub fn key(&self) -> &Vec<u8> {
        &self.0
    }
}