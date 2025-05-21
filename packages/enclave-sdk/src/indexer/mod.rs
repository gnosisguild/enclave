pub mod models;
use alloy::providers::Provider;
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;

use async_trait::async_trait;
use eyre::Result;
use models::E3;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::RwLock;

use crate::evm::{
    contracts::{
        EnclaveContract, EnclaveContractFactory, EnclaveRead, EnclaveReadOnlyProvider, ReadOnly,
    },
    events::{E3Activated, InputPublished},
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
}

#[derive(Clone)]
pub struct EnclaveIndexer<Store: DataStore> {
    listener: EventListener,
    contract: EnclaveContract<EnclaveReadOnlyProvider, ReadOnly>,
    store: Arc<RwLock<Store>>,
    contract_address: String,
    chain_id: u64,
}

impl<Store: DataStore> EnclaveIndexer<Store> {
    pub async fn new(
        ws_url: &str,
        contract_address: &str,
        store: Arc<RwLock<Store>>,
    ) -> Result<Self> {
        let listener = EventListener::create_contract_listener(ws_url, contract_address).await?;
        let contract = EnclaveContractFactory::create_read(ws_url, contract_address).await?;
        let chain_id = contract.provider.get_chain_id().await?;
        Ok(Self {
            store,
            contract,
            listener,
            contract_address: contract_address.to_string(),
            chain_id,
        })
    }

    pub async fn init_e3_activated(&mut self) -> Result<()> {
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
                    let e3_id = e.e3Id.to::<u64>();
                    let e3 = contract.get_e3(e.e3Id).await?;
                    let e3_obj = E3 {
                        chain_id,
                        ciphertext_inputs: vec![],
                        ciphertext_output: vec![],
                        committee_public_key: e.committeePublicKey.to_vec(),
                        duration: e3.duration.to::<u64>(),
                        e3_params: e3.e3ProgramParams.to_vec(),
                        enclave_address,
                        encryption_scheme_id: e3.encryptionSchemeId.to_vec(),
                        expiration: e.expiration.to::<u64>(),
                        id: e3_id,
                        plaintext_output: vec![],
                        request_block: e3.requestBlock.to::<u64>(),
                        seed: e3.seed.to::<u64>(), // TODO: make this into a bytes32
                        start_window: e3.startWindow.map(|n| n.to::<u64>()),
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

    pub async fn init_input_published(&mut self) -> Result<()> {
        let store = self.store.clone();
        self.listener
            .add_event_handler(move |e: InputPublished| {
                let store = store.clone();
                async move {
                    println!("InputPublished:{:?}", e);
                    let e3_id = e.e3Id.to::<u64>();
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

    pub async fn initialize(&mut self) -> Result<()> {
        self.init_e3_activated().await?;
        self.init_input_published().await?;
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        self.listener.listen().await
    }

    pub fn get_store(&self) -> Arc<RwLock<Store>> {
        self.store.clone()
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
