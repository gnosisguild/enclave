use super::models::E3;
use alloy::primitives::Uint;
use alloy::providers::Provider;
use alloy::sol_types::SolEvent;
use async_trait::async_trait;
use eyre::eyre;
use eyre::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::evm::{
    contracts::{EnclaveContract, EnclaveContractFactory, EnclaveRead, ReadOnly},
    events::{CiphertextOutputPublished, E3Activated, InputPublished, PlaintextOutputPublished},
    listener::EventListener,
};

type E3Id = u64;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("E3 not found: {0}")]
    E3NotFound(E3Id),
    #[error("Object not serializable: {0}")]
    Serialization(E3Id),
}

/// Trait for injectable DataStore. Note the implementor must manage interior mutability
#[async_trait]
pub trait DataStore: Send + Sync + 'static {
    type Error;
    async fn insert<T: Serialize + Send + Sync>(
        &mut self,
        key: &str,
        value: &T,
    ) -> Result<(), Self::Error>;
    async fn get<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>, Self::Error>;
    async fn modify<T, F>(&mut self, key: &str, f: F) -> Result<Option<T>, Self::Error>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: FnOnce(Option<T>) -> Option<T> + Send;
}

pub struct InMemoryStore {
    data: HashMap<String, Vec<u8>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

#[async_trait]
impl DataStore for InMemoryStore {
    type Error = eyre::Error;

    async fn insert<T: Serialize + Send + Sync>(
        &mut self,
        key: &str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.data
            .insert(key.to_string(), bincode::serialize(value)?);
        Ok(())
    }

    async fn get<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>, Self::Error> {
        Ok(self
            .data
            .get(key)
            .map(|bytes| bincode::deserialize(bytes))
            .transpose()?)
    }

    async fn modify<T, F>(&mut self, key: &str, f: F) -> Result<Option<T>, Self::Error>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: FnOnce(Option<T>) -> Option<T> + Send,
    {
        let current = self
            .data
            .get(key)
            .and_then(|bytes| bincode::deserialize(bytes).ok());

        match f(current) {
            Some(new_value) => {
                self.data
                    .insert(key.to_string(), bincode::serialize(&new_value)?);
                Ok(Some(new_value))
            }
            None => {
                self.data.remove(key);
                Ok(None)
            }
        }
    }
}

pub struct StoreWrapper<S> {
    inner: Arc<RwLock<S>>,
}

impl<S: DataStore> Clone for StoreWrapper<S> {
    fn clone(&self) -> Self {
        StoreWrapper {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<S: DataStore> StoreWrapper<S> {
    pub fn new(inner: Arc<RwLock<S>>) -> StoreWrapper<S> {
        Self { inner }
    }
}

#[async_trait]
impl<S: DataStore> DataStore for StoreWrapper<S> {
    type Error = S::Error;
    async fn insert<T: Serialize + Send + Sync>(
        &mut self,
        key: &str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.inner.write().await.insert(key, value).await
    }

    async fn get<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>, Self::Error> {
        self.inner.read().await.get(key).await
    }

    async fn modify<T, F>(&mut self, key: &str, f: F) -> Result<Option<T>, Self::Error>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: FnOnce(Option<T>) -> Option<T> + Send,
    {
        self.inner.write().await.modify(key, f).await
    }
}

#[derive(Clone)]
pub struct EnclaveIndexer<S: DataStore> {
    listener: EventListener,
    contract: EnclaveContract<ReadOnly>,
    store: Arc<RwLock<S>>,
    contract_address: String,
    chain_id: u64,
}

impl<S: DataStore> EnclaveIndexer<S> {
    pub async fn new(ws_url: &str, contract_address: &str, store: S) -> Result<Self> {
        let listener = EventListener::create_contract_listener(ws_url, contract_address).await?;
        let contract = EnclaveContractFactory::create_read(ws_url, contract_address).await?;
        let chain_id = contract.provider.get_chain_id().await?;
        let mut instance = Self {
            store: Arc::new(RwLock::new(store)),
            contract,
            listener,
            contract_address: contract_address.to_string(),
            chain_id,
        };
        instance.setup_listeners().await?;
        Ok(instance)
    }

    pub async fn add_event_handler<E, F, Fut>(&mut self, handler: F)
    where
        E: SolEvent + Send + Clone + 'static,
        F: Fn(E, StoreWrapper<S>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let store = StoreWrapper::new(self.store.clone());
        let handler = Arc::new(handler);
        self.listener
            .add_event_handler(move |e: E| {
                let handler = Arc::clone(&handler);
                let store = store.clone();
                async move { handler(e, store).await }
            })
            .await;
    }

    async fn capture_e3_activated(&mut self) -> Result<()> {
        let db = self.store.clone();
        let contract = self.contract.clone();
        let chain_id = self.chain_id;
        let enclave_address = self.contract_address.clone();
        self.listener
            .add_event_handler(move |e: E3Activated| {
                let db = db.clone();
                let enclave_address = enclave_address.clone();
                let contract = contract.clone();
                async move {
                    println!("E3Activated:{:?}", e);
                    let e3_id = u64_try_from(e.e3Id)?;
                    let e3 = contract.get_e3(e.e3Id).await?;
                    let e3_obj = E3 {
                        chain_id,
                        ciphertext_inputs: vec![],
                        ciphertext_output: vec![],
                        committee_public_key: e.committeePublicKey.to_vec(),
                        duration: u64_try_from(e3.duration)?,
                        e3_params: e3.e3ProgramParams.to_vec(),
                        enclave_address,
                        encryption_scheme_id: e3.encryptionSchemeId.to_vec(),
                        expiration: u64_try_from(e.expiration)?,
                        id: e3_id,
                        plaintext_output: vec![],
                        request_block: u64_try_from(e3.requestBlock)?,
                        seed: u64_try_from(e3.seed)?, // TODO: make this into a bytes32
                        start_window: [
                            u64_try_from(e3.startWindow[0])?,
                            u64_try_from(e3.startWindow[1])?,
                        ],
                        threshold: e3.threshold,
                    };

                    let key = format!("e3:{}", e3_id);

                    db.write()
                        .await
                        .insert(&key, &e3_obj)
                        .await
                        .map_err(|_| IndexerError::Serialization(e3_id))?;

                    Ok(())
                }
            })
            .await;
        Ok(())
    }

    async fn capture_input_published(&mut self) -> Result<()> {
        let store = self.store.clone();
        self.listener
            .add_event_handler(move |e: InputPublished| {
                let store = store.clone();
                async move {
                    println!("InputPublished:{:?}", e);
                    let e3_id = u64_try_from(e.e3Id)?;
                    let (mut e3, key) = get_e3(store.clone(), e3_id).await?;
                    e3.ciphertext_inputs
                        .push((e.data.to_vec(), e.index.to::<u64>()));
                    store
                        .write()
                        .await
                        .insert(&key, &e3)
                        .await
                        .map_err(|_| IndexerError::Serialization(e3_id))?;

                    Ok(())
                }
            })
            .await;
        Ok(())
    }

    async fn capture_ciphertext_output_published(&mut self) -> Result<()> {
        let store = self.store.clone();
        self.listener
            .add_event_handler(move |e: CiphertextOutputPublished| {
                let store = store.clone();
                async move {
                    println!("CiphertextOutputPublished:{:?}", e);
                    let e3_id = u64_try_from(e.e3Id)?;
                    let (mut e3, key) = get_e3(store.clone(), e3_id).await?;
                    e3.ciphertext_output = e.ciphertextOutput.to_vec();

                    store
                        .write()
                        .await
                        .insert(&key, &e3)
                        .await
                        .map_err(|_| IndexerError::Serialization(e3_id))?;

                    Ok(())
                }
            })
            .await;
        Ok(())
    }

    async fn capture_plaintext_output_published(&mut self) -> Result<()> {
        let store = self.store.clone();
        self.listener
            .add_event_handler(move |e: PlaintextOutputPublished| {
                let store = store.clone();
                async move {
                    println!("PlaintextOutputPublished:{:?}", e);
                    let e3_id = u64_try_from(e.e3Id)?;
                    let (mut e3, key) = get_e3(store.clone(), e3_id).await?;
                    e3.plaintext_output = e.plaintextOutput.to_vec();

                    store
                        .write()
                        .await
                        .insert(&key, &e3)
                        .await
                        .map_err(|_| IndexerError::Serialization(e3_id))?;

                    Ok(())
                }
            })
            .await;
        Ok(())
    }

    async fn setup_listeners(&mut self) -> Result<()> {
        self.capture_e3_activated().await?;
        self.capture_input_published().await?;
        self.capture_ciphertext_output_published().await?;
        self.capture_plaintext_output_published().await?;
        Ok(())
    }

    pub fn start(&self) -> Result<JoinHandle<()>> {
        let listener = self.listener.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = listener.listen().await {
                eprintln!("Error: {}", e);
            }
        });
        Ok(handle)
    }

    pub async fn get_e3(&self, e3_id: u64) -> Result<E3, IndexerError> {
        let (e3, _) = get_e3(self.store.clone(), e3_id).await?;
        Ok(e3)
    }

    pub fn get_listener(&self) -> EventListener {
        self.listener.clone()
    }

    pub fn get_store(&self) -> StoreWrapper<S> {
        StoreWrapper::new(self.store.clone())
    }
}

pub async fn get_e3(
    store: Arc<RwLock<impl DataStore>>,
    e3_id: u64,
) -> Result<(E3, String), IndexerError> {
    let key = format!("e3:{}", e3_id);
    match store
        .read()
        .await
        .get::<E3>(&key)
        .await
        .map_err(|_| IndexerError::Serialization(e3_id))?
    {
        Some(e3) => Ok((e3, key)),
        None => Err(IndexerError::E3NotFound(e3_id)),
    }
}

fn u64_try_from(input: Uint<256, 4>) -> Result<u64> {
    u64::try_from(input).map_err(|_| eyre!("larger than 64-bit"))
}
