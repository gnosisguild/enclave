use super::models::E3;
use async_trait::async_trait;
use enclave_sdk::indexer::DataStore;
use log::error;
use once_cell::sync::Lazy;
use rand::Rng;
use serde::{de::DeserializeOwned, Serialize};
use sled::Db;
use std::{error::Error, str, sync::Arc};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("SledDB error: {0}")]
    SledDB(#[from] sled::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
#[derive(Clone)]
pub struct SledDB {
    pub db: Db,
}
impl SledDB {
    pub fn new(path: &str) -> Result<Self, DatabaseError> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }
}

#[async_trait]
impl DataStore for SledDB {
    type Error = DatabaseError;

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
        // Edit in place
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

static GLOBAL_DB: Lazy<RwLock<SledDB>> = Lazy::new(|| {
    let pathdb = std::env::current_dir().unwrap().join("database/server");
    RwLock::new(SledDB::new(pathdb.to_str().unwrap()).unwrap())
});

pub async fn db_insert<T: Serialize + Send + Sync>(
    key: &str,
    value: &T,
) -> Result<(), DatabaseError> {
    let mut db = GLOBAL_DB.write().await;
    db.insert(key, value).await?;
    Ok(())
}

pub async fn db_get<T: DeserializeOwned + Send + Sync>(
    key: &str,
) -> Result<Option<T>, DatabaseError> {
    let db = GLOBAL_DB.read().await;
    db.get::<T>(key).await
}

pub async fn get_e3(e3_id: u64) -> Result<(E3, String), Box<dyn Error + Send + Sync>> {
    let key = format!("e3:{}", e3_id);
    match db_get::<E3>(&key).await? {
        Some(e3) => Ok((e3, key)),
        None => {
            error!("E3 state not found for key: {}", key);
            Err("E3 state not found".into())
        }
    }
}

pub async fn update_e3_status(
    e3_id: u64,
    status: String,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let key = format!("e3:{}", e3_id);
    let mut e3 = db_get::<E3>(&key).await?.unwrap();
    e3.status = status;
    db_insert(&key, &e3).await?;
    Ok(())
}

pub fn generate_emoji() -> [String; 2] {
    let emojis = [
        "🍇", "🍈", "🍉", "🍊", "🍋", "🍌", "🍍", "🥭", "🍎", "🍏", "🍐", "🍑", "🍒", "🍓", "🫐",
        "🥝", "🍅", "🫒", "🥥", "🥑", "🍆", "🥔", "🥕", "🌽", "🌶️", "🫑", "🥒", "🥬", "🥦", "🧄",
        "🧅", "🍄", "🥜", "🫘", "🌰", "🍞", "🥐", "🥖", "🫓", "🥨", "🥯", "🥞", "🧇", "🧀", "🍖",
        "🍗", "🥩", "🥓", "🍔", "🍟", "🍕", "🌭", "🥪", "🌮", "🌯", "🫔", "🥙", "🧆", "🥚", "🍳",
        "🥘", "🍲", "🫕", "🥣", "🥗", "🍿", "🧈", "🧂", "🥫", "🍱", "🍘", "🍙", "🍚", "🍛", "🍜",
        "🍝", "🍠", "🍢", "🍣", "🍤", "🍥", "🥮", "🍡", "🥟", "🥠", "🥡", "🦀", "🦞", "🦐", "🦑",
        "🦪", "🍦", "🍧", "🍨", "🍩", "🍪", "🎂", "🍰", "🧁", "🥧", "🍫", "🍬", "🍭", "🍮", "🍯",
        "🍼", "🥛", "☕", "🍵", "🍾", "🍷", "🍸", "🍹", "🍺", "🍻", "🥂", "🥃",
    ];
    let mut index1 = rand::thread_rng().gen_range(0..emojis.len());
    let index2 = rand::thread_rng().gen_range(0..emojis.len());
    if index1 == index2 {
        if index1 == emojis.len() {
            index1 = index1 - 1;
        } else {
            index1 = index1 + 1;
        };
    };
    [emojis[index1].to_string(), emojis[index2].to_string()]
}
