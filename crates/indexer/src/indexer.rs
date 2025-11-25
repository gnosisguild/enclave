// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::E3Repository;

use super::{models::E3, DataStore};
use alloy::hex;
use alloy::primitives::Uint;
use alloy::providers::Provider;
use alloy::sol_types::SolEvent;
use async_trait::async_trait;
use e3_evm_helpers::{
    block_listener::BlockListener,
    contracts::{
        EnclaveContract, EnclaveContractFactory, EnclaveRead, ProviderType, ReadOnly, ReadWrite,
    },
    event_listener::EventListener,
    events::{CiphertextOutputPublished, E3Activated, PlaintextOutputPublished},
};
use eyre::eyre;
use eyre::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

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

#[derive(Clone)]
pub struct EnclaveIndexer<S: DataStore, R: ProviderType> {
    ctx: Arc<IndexerContext<S, R>>,
}

impl<S: DataStore, R: ProviderType> Drop for EnclaveIndexer<S, R> {
    fn drop(&mut self) {
        info!("EnclaveIndexer is DROPPED");
    }
}

pub struct IndexerContext<S: DataStore, R: ProviderType> {
    store: SharedStore<S>,
    event_listener: EventListener,
    block_listener: BlockListener,
    contract: EnclaveContract<R>,
    contract_address: String,
    chain_id: u64,
}

impl<S: DataStore, R: ProviderType> IndexerContext<S, R> {
    pub fn store(&self) -> SharedStore<S> {
        self.store.clone()
    }

    pub fn event_listener(&self) -> EventListener {
        self.event_listener.clone()
    }

    pub fn block_listener(&self) -> BlockListener {
        self.block_listener.clone()
    }

    pub fn contract(&self) -> EnclaveContract<R> {
        self.contract.clone()
    }
    pub fn enclave_address(&self) -> String {
        self.contract_address.clone()
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }
}

impl<R: ProviderType> EnclaveIndexer<InMemoryStore, R> {
    pub async fn new_with_in_mem_store(
        event_listener: EventListener,
        contract: EnclaveContract<R>,
    ) -> Result<EnclaveIndexer<InMemoryStore, R>> {
        let store = InMemoryStore::new();

        EnclaveIndexer::new(event_listener, contract, store).await
    }
}

impl EnclaveIndexer<InMemoryStore, ReadOnly> {
    /// Creates an `EnclaveIndexer` with an in-memory store.
    ///
    /// Note: `addresses[0]` must be the enclave contract address.
    pub async fn from_endpoint_address_in_mem(ws_url: &str, addresses: &[&str]) -> Result<Self> {
        let event_listener = EventListener::create_contract_listener(ws_url, addresses).await?;
        let contract = EnclaveContractFactory::create_read(ws_url, addresses[0]).await?;
        EnclaveIndexer::<InMemoryStore, ReadOnly>::new_with_in_mem_store(event_listener, contract)
            .await
    }

    /// Creates an `EnclaveIndexer` with a provided in-memory store.
    ///
    /// Note: `addresses[0]` must be the enclave contract address.
    pub async fn from_endpoint_address(
        ws_url: &str,
        addresses: &[&str],
        store: InMemoryStore,
    ) -> Result<Self> {
        let event_listener = EventListener::create_contract_listener(ws_url, addresses).await?;
        let contract = EnclaveContractFactory::create_read(ws_url, addresses[0]).await?;
        EnclaveIndexer::new(event_listener, contract, store).await
    }
}

impl<S: DataStore> EnclaveIndexer<S, ReadWrite> {
    /// Creates a new EnclaveIndexer with a writeable contract.
    pub async fn new_with_write_contract(
        ws_url: &str,
        addresses: &[&str], // First address must be contract_address
        store: S,
        private_key: &str,
    ) -> Result<Self> {
        let Some(contract_address) = addresses.first() else {
            return Err(eyre::eyre!("No addresses provided"));
        };
        let event_listener = EventListener::create_contract_listener(ws_url, addresses).await?;
        EnclaveIndexer::new(
            event_listener,
            EnclaveContractFactory::create_write(ws_url, contract_address, private_key).await?,
            store,
        )
        .await
    }
}

impl<S: DataStore, R: ProviderType> EnclaveIndexer<S, R> {
    pub async fn new(
        event_listener: EventListener,
        contract: EnclaveContract<R>,
        store: S,
    ) -> Result<Self> {
        let chain_id = contract.provider.get_chain_id().await?;
        let contract_address = contract.address().to_string();
        let block_listener = BlockListener::new(event_listener.provider());
        let mut instance = Self {
            ctx: Arc::new(IndexerContext {
                store: SharedStore::new(Arc::new(RwLock::new(store))),
                contract,
                event_listener,
                block_listener,
                contract_address,
                chain_id,
            }),
        };
        instance.setup_listeners().await?;
        info!("EnclaveIndexer has been configured");
        Ok(instance)
    }

    /// Listen for contract events from all contracts.
    /// Callback will provide the event and a context object.
    pub async fn add_event_handler<E, F, Fut>(&self, handler: F)
    where
        E: SolEvent + Send + Clone + 'static,
        F: Fn(E, Arc<IndexerContext<S, R>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(handler);
        let ctx = self.ctx.clone();
        // In order to avoid a memory leak we create a weak reference here
        let ctx_weak = Arc::downgrade(&ctx);

        self.ctx
            .event_listener
            .add_event_handler(move |e: E| {
                let handler = Arc::clone(&handler);
                let ctx_weak = ctx_weak.clone();

                async move {
                    // We check the weak reference if it can be upgraded
                    // if not it must have been destroyed
                    if let Some(ctx) = ctx_weak.upgrade() {
                        handler(e, ctx).await
                    } else {
                        warn!("Context was dropped!");
                        Ok(())
                    }
                }
            })
            .await;
    }

    async fn register_e3_activated(&mut self) -> Result<()> {
        self.add_event_handler(move |e: E3Activated, ctx| {
            async move {
                let contract = ctx.contract();
                let db = ctx.store();
                let enclave_address = ctx.enclave_address();
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
                    chain_id: ctx.chain_id(),
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

    async fn register_ciphertext_output_published(&mut self) -> Result<()> {
        self.add_event_handler(move |e: CiphertextOutputPublished, ctx| async move {
            let store = ctx.store();
            info!(
                "CiphertextOutputPublished: e3_id={}, output=0x{}...",
                e.e3Id,
                hex::encode(&e.ciphertextOutput[..8.min(e.ciphertextOutput.len())])
            );
            let e3_id = u64_try_from(e.e3Id)?;

            let mut repo = E3Repository::new(store, e3_id);
            repo.set_ciphertext_output(e.ciphertextOutput.to_vec())
                .await?;

            Ok(())
        })
        .await;
        Ok(())
    }

    async fn register_plaintext_output_published(&mut self) -> Result<()> {
        self.add_event_handler(move |e: PlaintextOutputPublished, ctx| async move {
            let store = ctx.store();
            info!(
                "PlaintextOutputPublished: e3_id={}, output=0x{}...",
                e.e3Id,
                hex::encode(&e.plaintextOutput[..8.min(e.plaintextOutput.len())])
            );
            let e3_id = u64_try_from(e.e3Id)?;
            let mut repo = E3Repository::new(store, e3_id);
            repo.set_plaintext_output(e.plaintextOutput.to_vec())
                .await?;

            Ok(())
        })
        .await;
        Ok(())
    }

    async fn setup_listeners(&mut self) -> Result<()> {
        info!("Setting up listeners for EnclaveIndexer...");
        self.register_e3_activated().await?;
        self.register_ciphertext_output_published().await?;
        self.register_plaintext_output_published().await?;
        info!("Listeners have been setup!");
        Ok(())
    }

    pub async fn listen(&self) -> Result<()> {
        info!("Starting EnclaveIndexer listening...");
        tokio::select! {
            res = self.ctx.event_listener.listen() => {
                match res {
                    Ok(_) => warn!("EventListener curiously halted naturally."),
                    Err(e) => error!("EventListener halted with an error: {e}")
                }
            }
            res = self.ctx.block_listener.listen() => {
                match res {
                    Ok(_) => warn!("BlockListener curiously halted naturally."),
                    Err(e) => error!("BlockListener halted with an error: {e}")
                }
            }
        }
        Ok(())
    }

    pub async fn get_e3(&self, e3_id: u64) -> Result<E3, IndexerError> {
        let (e3, _) = get_e3(self.ctx.store.inner.clone(), e3_id).await?;
        Ok(e3)
    }

    pub fn get_store(&self) -> SharedStore<S> {
        self.ctx.store.clone()
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
