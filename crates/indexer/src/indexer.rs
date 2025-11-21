// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::callback_queue::CallbackQueue;
use crate::E3Repository;

use super::{models::E3, DataStore};
use alloy::primitives::Uint;
use alloy::providers::Provider;
use alloy::sol_types::SolEvent;
use alloy::{consensus::BlockHeader, hex};
use async_trait::async_trait;
use e3_evm_helpers::{
    contracts::{EnclaveContract, EnclaveContractFactory, EnclaveRead, ReadOnly},
    events::{CiphertextOutputPublished, E3Activated, InputPublished, PlaintextOutputPublished},
    listener::EventListener,
};
// TODO: Remove eyre in favour of thiserror
use eyre::eyre;
use eyre::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

type E3Id = u64;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("E3 not found: {0}")]
    E3NotFound(E3Id),
    #[error("Object not serializable: {0}")]
    Serialization(E3Id),
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

    async fn modify<T, F>(&mut self, key: &str, mut f: F) -> Result<Option<T>, Self::Error>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: FnMut(Option<T>) -> Option<T> + Send,
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

pub struct SharedStore<S> {
    inner: Arc<RwLock<S>>,
}

impl<S: DataStore> Clone for SharedStore<S> {
    fn clone(&self) -> Self {
        SharedStore {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<S: DataStore> SharedStore<S> {
    pub fn new(inner: Arc<RwLock<S>>) -> SharedStore<S> {
        Self { inner }
    }
}

#[async_trait]
impl<S: DataStore> DataStore for SharedStore<S> {
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
        F: FnMut(Option<T>) -> Option<T> + Send,
    {
        self.inner.write().await.modify(key, f).await
    }
}

/// Stores E3 event data on a datastore (persisted or otherwise) for easy querying.
pub struct EnclaveIndexer<S> {
    listener: EventListener,
    callbacks: CallbackQueue,
    contract: EnclaveContract<ReadOnly>,
    store: Arc<RwLock<S>>,
    contract_address: String,
    chain_id: u64,
}

impl<S> Clone for EnclaveIndexer<S> {
    fn clone(&self) -> Self {
        Self {
            listener: self.listener.clone(),
            callbacks: self.callbacks.clone(),
            contract: self.contract.clone(),
            store: self.store.clone(),
            contract_address: self.contract_address.clone(),
            chain_id: self.chain_id,
        }
    }
}

impl EnclaveIndexer<InMemoryStore> {
    pub async fn new_with_in_mem_store(
        listener: EventListener,
        contract: EnclaveContract<ReadOnly>,
    ) -> Result<EnclaveIndexer<InMemoryStore>> {
        let store = InMemoryStore::new();

        EnclaveIndexer::new(listener, contract, store).await
    }

    pub async fn from_endpoint_address_in_mem(
        ws_url: &str,
        contract_address: &str,
    ) -> Result<EnclaveIndexer<InMemoryStore>> {
        let listener = EventListener::create_contract_listener(ws_url, contract_address).await?;
        let contract = EnclaveContractFactory::create_read(ws_url, contract_address).await?;
        EnclaveIndexer::<InMemoryStore>::new_with_in_mem_store(listener, contract).await
    }
}

impl<S: DataStore> EnclaveIndexer<S> {
    /// Try to create a new EnclaveIndexer
    pub async fn new(
        listener: EventListener,
        contract: EnclaveContract<ReadOnly>,
        store: S,
    ) -> Result<Self> {
        let chain_id = contract.provider.get_chain_id().await?;
        let contract_address = contract.address().to_string();
        let mut instance = Self {
            store: Arc::new(RwLock::new(store)),
            contract,
            callbacks: CallbackQueue::new(),
            listener,
            contract_address,
            chain_id,
        };
        instance.setup_listeners().await?;
        Ok(instance)
    }

    /// Try to create a new EnclaveIndexer from an endpoint and an address
    pub async fn from_endpoint_address(
        ws_url: &str,
        contract_address: &str,
        store: S,
    ) -> Result<Self> {
        let listener = EventListener::create_contract_listener(ws_url, contract_address).await?;
        let contract = EnclaveContractFactory::create_read(ws_url, contract_address).await?;
        EnclaveIndexer::new(listener, contract, store).await
    }

    /// Add a new Solidity event handler to the indexer
    pub async fn add_event_handler<E, F, Fut>(&mut self, handler: F)
    where
        E: SolEvent + Send + Clone + 'static,
        F: Fn(E, SharedStore<S>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let store = SharedStore::new(self.store.clone());
        let handler = Arc::new(handler);
        self.listener
            .add_event_handler(move |e: E| {
                let handler = Arc::clone(&handler);
                let store = store.clone();
                async move { handler(e, store).await }
            })
            .await;
    }

    /// Register a callback for execution after the given timestap as returned by the blockchain.
    pub fn dispatch_after_timestamp<F, Fut>(&mut self, when: u64, callback: F)
    where
        F: Fn(SharedStore<S>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let store = SharedStore::new(self.store.clone());
        let callback = Arc::new(callback);

        self.callbacks.push(when, move || {
            let callback = Arc::clone(&callback);
            let store = store.clone();
            callback(store)
        });
    }

    /// Start listening
    pub fn start(&self) {
        self.listener.start()
    }

    /// Get E3 data by ID
    pub async fn get_e3(&self, e3_id: u64) -> Result<E3, IndexerError> {
        let (e3, _) = get_e3(self.store.clone(), e3_id).await?;
        Ok(e3)
    }

    /// Get a handle to the listener
    pub fn get_listener(&self) -> EventListener {
        self.listener.clone()
    }

    /// Get a handle to the store
    pub fn get_store(&self) -> SharedStore<S> {
        SharedStore::new(self.store.clone())
    }

    async fn register_e3_activated(&mut self) -> Result<()> {
        let db = self.store.clone();
        let contract = self.contract.clone();
        let chain_id = self.chain_id;
        let enclave_address = self.contract_address.clone();
        self.listener
            .add_event_handler(move |e: E3Activated| {
                let db = SharedStore::new(db.clone());
                let enclave_address = enclave_address.clone();
                let contract = contract.clone();
                async move {
                    println!(
                        "E3Activated: id={}, expiration={}, pubkey=0x{}...",
                        e.e3Id,
                        e.expiration,
                        hex::encode(&e.committeePublicKey[..8.min(e.committeePublicKey.len())])
                    );
                    let e3_id = u64_try_from(e.e3Id)?;
                    let e3 = contract.get_e3(e.e3Id).await?;
                    let duration = u64_try_from(e3.duration)?;
                    let expiration = u64_try_from(e.expiration)?;
                    let seed = e3.seed.to_be_bytes();
                    let request_block = u64_try_from(e3.requestBlock)?;
                    let start_window = [
                        u64_try_from(e3.startWindow[0])?,
                        u64_try_from(e3.startWindow[1])?,
                    ];

                    // NOTE: we are only saving protocol specific info
                    // here and not CRISP specific info so E3 corresponds to the solidity E3
                    let e3_obj = E3 {
                        chain_id,
                        ciphertext_inputs: vec![],
                        ciphertext_output: vec![],
                        committee_public_key: e.committeePublicKey.to_vec(),
                        duration,
                        custom_params: e3.customParams.to_vec(),
                        e3_params: e3.e3ProgramParams.to_vec(),
                        enclave_address,
                        encryption_scheme_id: e3.encryptionSchemeId.to_vec(),
                        expiration,
                        id: e3_id,
                        plaintext_output: vec![],
                        request_block,
                        seed,
                        start_window,
                        threshold: e3.threshold,
                    };

                    let mut repo = E3Repository::new(db, e3_id);

                    repo.set_e3(e3_obj).await?;
                    Ok(())
                }
            })
            .await;
        Ok(())
    }

    async fn register_input_published(&mut self) -> Result<()> {
        let store = self.store.clone();
        self.listener
            .add_event_handler(move |e: InputPublished| {
                let store = SharedStore::new(store.clone());
                async move {
                    println!(
                        "InputPublished: e3_id={}, index={}, data=0x{}...",
                        e.e3Id,
                        e.index,
                        hex::encode(&e.data[..8.min(e.data.len())])
                    );
                    let e3_id = u64_try_from(e.e3Id)?;

                    let mut repo = E3Repository::new(store, e3_id);
                    repo.insert_ciphertext_input(e.data.to_vec(), e.index.to::<u64>())
                        .await?;
                    Ok(())
                }
            })
            .await;
        Ok(())
    }

    async fn register_ciphertext_output_published(&mut self) -> Result<()> {
        let store = self.store.clone();
        self.listener
            .add_event_handler(move |e: CiphertextOutputPublished| {
                let store = SharedStore::new(store.clone());
                async move {
                    println!(
                        "CiphertextOutputPublished: e3_id={}, output=0x{}...",
                        e.e3Id,
                        hex::encode(&e.ciphertextOutput[..8.min(e.ciphertextOutput.len())])
                    );
                    let e3_id = u64_try_from(e.e3Id)?;

                    let mut repo = E3Repository::new(store, e3_id);
                    repo.set_ciphertext_output(e.ciphertextOutput.to_vec())
                        .await?;

                    Ok(())
                }
            })
            .await;
        Ok(())
    }

    async fn register_plaintext_output_published(&mut self) -> Result<()> {
        let store = self.store.clone();
        self.listener
            .add_event_handler(move |e: PlaintextOutputPublished| {
                let store = SharedStore::new(store.clone());
                async move {
                    println!(
                        "PlaintextOutputPublished: e3_id={}, output=0x{}...",
                        e.e3Id,
                        hex::encode(&e.plaintextOutput[..8.min(e.plaintextOutput.len())])
                    );
                    let e3_id = u64_try_from(e.e3Id)?;
                    let mut repo = E3Repository::new(store, e3_id);
                    repo.set_plaintext_output(e.plaintextOutput.to_vec())
                        .await?;

                    Ok(())
                }
            })
            .await;
        Ok(())
    }

    async fn register_blocktime_callback_handler(&mut self) -> Result<()> {
        info!("register_blocktime_callback_handler()...");
        let callbacks = self.callbacks.clone();
        self.listener
            .add_block_handler(move |block| {
                let timestamp = block.timestamp();
                let blockheight = block.number();
                let callbacks = callbacks.clone();
                async move {
                    info!("on block: {}:{}", blockheight, timestamp);
                    callbacks.execute_until_including(timestamp).await?;
                    Ok(())
                }
            })
            .await;
        Ok(())
    }

    async fn setup_listeners(&mut self) -> Result<()> {
        self.register_e3_activated().await?;
        self.register_input_published().await?;
        self.register_ciphertext_output_published().await?;
        self.register_plaintext_output_published().await?;
        self.register_blocktime_callback_handler().await?;
        Ok(())
    }
}

pub async fn get_e3(
    store: Arc<RwLock<impl DataStore>>,
    e3_id: u64,
) -> Result<(E3, String), IndexerError> {
    let key = format!("_e3:{}", e3_id);
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
