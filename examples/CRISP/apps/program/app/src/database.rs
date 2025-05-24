use async_trait::async_trait;
use enclave_sdk::indexer::DataStore;
use eyre::Result;
use serde::{de::DeserializeOwned, Serialize};
use sled::Db;

#[derive(Clone)]
pub struct SledDB {
    pub db: Db,
}

impl SledDB {
    pub fn new(path: &str) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }
}

#[async_trait]
impl DataStore for SledDB {
    type Error = eyre::Error;
    async fn insert<T: Serialize + Send + Sync>(
        &mut self,
        key: &str,
        value: &T,
    ) -> Result<(), Self::Error> {
        let serialized = serde_json::to_vec(value)?;
        self.db.insert(key.as_bytes(), serialized)?;
        Ok(())
    }

    async fn get<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>, Self::Error> {
        if let Some(bytes) = self.db.get(key.as_bytes())? {
            let value = serde_json::from_slice(&bytes)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    async fn modify<T, F>(&mut self, key: &str, mut f: F) -> Result<Option<T>, Self::Error>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: FnMut(Option<T>) -> Option<T> + Send,
    {
        // Edit in place atomically
        let result = self.db.update_and_fetch(key, |old_bytes| {
            let current_value = old_bytes.and_then(|bytes| serde_json::from_slice(bytes).ok());
            let new_value = f(current_value);
            new_value.and_then(|val| serde_json::to_vec(&val).ok())
        })?;

        // Deserialize the final result
        result
            .map(|bytes| serde_json::from_slice(&bytes))
            .transpose()
            .map_err(|e| e.into())
    }
}
