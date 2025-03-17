use super::models::E3;
use once_cell::sync::Lazy;
use rand::Rng;
use sled::Db;
use std::{error::Error, str, sync::Arc};
use tokio::sync::RwLock;
use log::error;
use thiserror::Error;
use serde::{Serialize, de::DeserializeOwned};

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("SledDB error: {0}")]
    SledDB(#[from] sled::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
#[derive(Clone)]
pub struct SledDB {
    pub db: Arc<RwLock<Db>>,
}

impl SledDB {
    pub fn new(path: &str) -> Result<Self, DatabaseError> {
        let db = sled::open(path)?;
        Ok(Self { db: Arc::new(RwLock::new(db)) })
    }

    pub async fn insert<T: Serialize>(&self, key: &str, value: &T) -> Result<(), DatabaseError> {
        let serialized = serde_json::to_vec(value)?;
        self.db.write().await.insert(key.as_bytes(), serialized)?;
        Ok(())
    }

    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, DatabaseError> {
        if let Some(bytes) = self.db.read().await.get(key.as_bytes())? {
            let value = serde_json::from_slice(&bytes)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}

pub static GLOBAL_DB: Lazy<SledDB> = Lazy::new(|| {
    let pathdb = std::env::current_dir()
        .unwrap()
        .join("database/server");
    SledDB::new(pathdb.to_str().unwrap()).unwrap()
});

pub async fn get_e3(e3_id: u64) -> Result<(E3, String), Box<dyn Error + Send + Sync>> {
    let key = format!("e3:{}", e3_id);
    match GLOBAL_DB.get::<E3>(&key).await? {
        Some(e3) => Ok((e3, key)),
        None => {
            error!("E3 state not found for key: {}", key);
            Err("E3 state not found".into())
        }
    }
}

pub async fn update_e3_status(e3_id: u64, status: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    let key = format!("e3:{}", e3_id);
    let mut e3 = GLOBAL_DB.get::<E3>(&key).await?.unwrap();
    e3.status = status;
    GLOBAL_DB.insert(&key, &e3).await?;
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
