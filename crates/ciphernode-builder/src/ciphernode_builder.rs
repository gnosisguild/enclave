// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::CiphernodeHandle;
use actix::{Actor, Addr};
use alloy::signers::{k256::ecdsa::SigningKey, local::LocalSigner};
use anyhow::Result;
use derivative::Derivative;
use e3_aggregator::ext::{
    PlaintextAggregatorExtension, PublicKeyAggregatorExtension,
    ThresholdPlaintextAggregatorExtension,
};
use e3_config::chain_config::ChainConfig;
use e3_crypto::Cipher;
use e3_data::{DataStore, InMemStore, Repositories, RepositoriesFactory};
use e3_events::{EnclaveEvent, EventBus, EventBusConfig};
use e3_evm::{
    helpers::{
        load_signer_from_repository, ConcreteReadProvider, ConcreteWriteProvider, EthProvider,
        ProviderConfig,
    },
    CiphernodeRegistryReaderRepositoryFactory, CiphernodeRegistrySol, EnclaveSol, EnclaveSolReader,
    EnclaveSolReaderRepositoryFactory, EthPrivateKeyRepositoryFactory, RegistryFilterSol,
};
use e3_fhe::ext::FheExtension;
use e3_keyshare::ext::{KeyshareExtension, ThresholdKeyshareExtension};
use e3_multithread::Multithread;
use e3_request::E3Router;
use e3_sortition::{CiphernodeSelector, Sortition, SortitionRepositoryFactory};
use e3_utils::{rand_eth_addr, SharedRng};
use std::{collections::HashMap, sync::Arc};
use tracing::info;

/// Build a ciphernode configuration.
// NOTE: We could use a typestate pattern here to separate production and testing methods. I hummed
// and hawed about it for quite a while and in the end felt it was too complex while we dont know
// the exact configurations we will use yet
#[derive(Derivative)]
#[derivative(Debug)]
pub struct CiphernodeBuilder {
    address: Option<String>,
    chains: Vec<ChainConfig>,
    #[derivative(Debug = "ignore")]
    cipher: Arc<Cipher>,
    contract_components: ContractComponents,
    datastore: Option<DataStore>,
    keyshare: Option<KeyshareKind>,
    logging: bool,
    multithread_cache: Option<Addr<Multithread>>,
    plaintext_agg: bool,
    pubkey_agg: bool,
    rng: SharedRng,
    source_bus: Option<BusMode<Addr<EventBus<EnclaveEvent>>>>,
    testmode_errors: bool,
    testmode_history: bool,
    threads: Option<usize>,
    threshold_plaintext_agg: bool,
}

#[derive(Default, Debug)]
pub struct ContractComponents {
    enclave_reader: bool,
    enclave: bool,
    registry_filter: bool,
    ciphernode_registry: bool,
}

#[derive(Clone, Debug)]
pub enum BusMode<T> {
    Forked(T),
    Source(T),
}

#[derive(Clone, Debug)]
pub enum KeyshareKind {
    Threshold,
    NonThreshold, // Soft Deprecated
}

impl CiphernodeBuilder {
    pub fn new(rng: SharedRng, cipher: Arc<Cipher>) -> Self {
        Self {
            address: None,
            chains: vec![],
            cipher,
            contract_components: ContractComponents::default(),
            datastore: None,
            keyshare: None,
            logging: false,
            multithread_cache: None,
            plaintext_agg: false,
            pubkey_agg: false,
            rng,
            source_bus: None,
            testmode_errors: false,
            testmode_history: false,
            threads: None,
            threshold_plaintext_agg: false,
        }
    }

    /// Use the given bus for all events. No new bus is created.
    pub fn with_source_bus(mut self, bus: &Addr<EventBus<EnclaveEvent>>) -> Self {
        self.source_bus = Some(BusMode::Source(bus.clone()));
        self
    }

    /// Fork all events from the given source bus. Events will be both broadcast on the source bus
    /// and a local bus created for this instance
    pub fn testmode_with_forked_bus(mut self, bus: &Addr<EventBus<EnclaveEvent>>) -> Self {
        self.source_bus = Some(BusMode::Forked(bus.clone()));
        self
    }

    /// Use the TrBFV feature
    pub fn with_trbfv(mut self) -> Self {
        self.keyshare = Some(KeyshareKind::Threshold);
        self
    }

    /// Use the Deprecated Keyshare feature
    #[deprecated = "in future versions we will migrate to with_trbfv()"]
    pub fn with_keyshare(mut self) -> Self {
        self.keyshare = Some(KeyshareKind::NonThreshold);
        self
    }

    /// Attach an existing in mem store to the node
    pub fn with_datastore(mut self, store: DataStore) -> Self {
        self.datastore = Some(store);
        self
    }

    /// Attach a history collecting test module.
    /// This is conspicuously named so we understand that this should only be used when testing
    pub fn testmode_with_history(mut self) -> Self {
        self.testmode_history = true;
        self
    }

    /// Attach an error collecting test module
    /// This is conspicuously named so we understand that this should only be used when testing
    pub fn testmode_with_errors(mut self) -> Self {
        self.testmode_errors = true;
        self
    }

    /// Use the node configuration on these specific chains. This will overwrite any previously
    /// given chains.
    pub fn with_chains(mut self, chains: &[ChainConfig]) -> Self {
        self.chains = chains.to_vec();
        self
    }

    /// Use the given Address to represent the node
    pub fn with_address(mut self, addr: &str) -> Self {
        self.address = Some(addr.to_owned());
        self
    }

    /// Log data actor events
    pub fn with_logging(mut self) -> Self {
        self.logging = true;
        self
    }

    /// Do public key aggregation
    pub fn with_pubkey_aggregation(mut self) -> Self {
        self.pubkey_agg = true;
        self
    }

    /// Do plaintext aggregation
    pub fn with_plaintext_aggregation(mut self) -> Self {
        self.plaintext_agg = true;
        self
    }

    /// Inject a preexisting multithread actor. This is mainly used for testing.
    pub fn with_injected_multithread(mut self, multithread: Addr<Multithread>) -> Self {
        self.multithread_cache = Some(multithread);
        self
    }

    /// Setup how many threads to use within the multithread actor
    #[deprecated(note = "This method is under construction and should not be used yet")]
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.threads = Some(threads);
        self
    }

    /// Setup a ThresholdPlaintextAggregator
    pub fn with_threshold_plaintext_aggregation(mut self) -> Self {
        self.threshold_plaintext_agg = true;
        self
    }

    /// Setup an Enclave contract reader for every evm chain provided
    pub fn with_contract_enclave_reader(mut self) -> Self {
        self.contract_components.enclave_reader = true;
        self
    }

    /// Setup an Enclave contract reader and writer for every evm chain provided
    pub fn with_contract_enclave_full(mut self) -> Self {
        self.contract_components.enclave = true;
        self
    }

    /// Setup a writable RegistryFilter for every evm chain provided
    pub fn with_contract_registry_filter(mut self) -> Self {
        self.contract_components.registry_filter = true;
        self
    }

    /// Setup a CiphernodeRegistry listener for every evm chain provided
    pub fn with_contract_ciphernode_registry(mut self) -> Self {
        self.contract_components.ciphernode_registry = true;
        self
    }

    fn create_local_bus() -> Addr<EventBus<EnclaveEvent>> {
        EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start()
    }

    pub async fn build(mut self) -> anyhow::Result<CiphernodeHandle> {
        // Local bus for ciphernode events can either be forked from a bus or it can be directly
        // attached to a source bus
        let local_bus = match self.source_bus {
            // Forked bus - pipe all events from the source to dest
            Some(BusMode::Forked(ref bus)) => {
                let local_bus = Self::create_local_bus();
                info!("Setting up Event pipe");
                EventBus::pipe(&bus, &local_bus);
                local_bus
            }
            // Source bus - simply attach to the source bus
            Some(BusMode::Source(ref bus)) => bus.clone(),
            // Nothing specified
            None => Self::create_local_bus(),
        };

        // History collector for taking historical events for analysis and testing
        let history = if self.testmode_history {
            info!("Setting up history collector");
            Some(EventBus::<EnclaveEvent>::history(&local_bus))
        } else {
            None
        };

        // Error collector for taking historical events for analysis and testing
        let errors = if self.testmode_errors {
            info!("Setting up error collector");
            Some(EventBus::<EnclaveEvent>::error(&local_bus))
        } else {
            None
        };

        let addr = if let Some(addr) = self.address.clone() {
            info!("Using eth address = {}", addr);
            addr
        } else {
            info!("Using random eth address");
            // TODO: This is for testing and should not be used for production if we use this to create ciphernodes in production
            rand_eth_addr(&self.rng)
        };

        let store = self
            .datastore
            .clone()
            .unwrap_or_else(|| (&InMemStore::new(self.logging).start()).into());

        let repositories = store.repositories();
        let sortition = Sortition::attach(&local_bus, repositories.sortition()).await?;

        // Ciphernode Selector
        CiphernodeSelector::attach(&local_bus, &sortition, &addr);

        let mut provider_cache = ProviderCaches::new();
        let cipher = &self.cipher;

        // TODO: gather an async handle from the event readers that closes when they shutdown and
        // join it with the network manager joinhandle below
        for chain in self
            .chains
            .iter()
            .filter(|chain| chain.enabled.unwrap_or(true))
        {
            if self.contract_components.enclave {
                let read_provider = provider_cache.ensure_read_provider(chain).await?;
                let write_provider = provider_cache
                    .ensure_write_provider(&repositories, chain, cipher)
                    .await?;
                EnclaveSol::attach(
                    &local_bus,
                    read_provider.clone(),
                    write_provider.clone(),
                    &chain.contracts.enclave.address(),
                    &repositories.enclave_sol_reader(read_provider.chain_id()),
                    chain.contracts.enclave.deploy_block(),
                    chain.rpc_url.clone(),
                )
                .await?;
            }

            if self.contract_components.enclave_reader {
                let read_provider = provider_cache.ensure_read_provider(chain).await?;
                EnclaveSolReader::attach(
                    &local_bus,
                    read_provider.clone(),
                    &chain.contracts.enclave.address(),
                    &repositories.enclave_sol_reader(read_provider.chain_id()),
                    chain.contracts.enclave.deploy_block(),
                    chain.rpc_url.clone(),
                )
                .await?;
            }

            if self.contract_components.registry_filter {
                let write_provider = provider_cache
                    .ensure_write_provider(&repositories, chain, cipher)
                    .await?;
                RegistryFilterSol::attach(
                    &local_bus,
                    write_provider.clone(),
                    &chain.contracts.filter_registry.address(),
                )
                .await?;
            }

            if self.contract_components.ciphernode_registry {
                let read_provider = provider_cache.ensure_read_provider(chain).await?;
                CiphernodeRegistrySol::attach(
                    &local_bus,
                    read_provider.clone(),
                    &chain.contracts.ciphernode_registry.address(),
                    &repositories.ciphernode_registry_reader(read_provider.chain_id()),
                    chain.contracts.ciphernode_registry.deploy_block(),
                    chain.rpc_url.clone(),
                )
                .await?;
            }
        }

        // E3 specific setup
        let mut e3_builder = E3Router::builder(&local_bus, store.clone());

        if let Some(KeyshareKind::Threshold) = self.keyshare {
            let multithread = self.ensure_multithread();
            info!("Setting up ThresholdKeyshareExtension");
            e3_builder = e3_builder.with(ThresholdKeyshareExtension::create(
                &local_bus,
                &self.cipher,
                &multithread,
                &addr,
            ))
        }

        if matches!(self.keyshare, Some(KeyshareKind::NonThreshold))
            || self.pubkey_agg
            || self.plaintext_agg
        {
            info!("Setting up FheExtension");
            e3_builder = e3_builder.with(FheExtension::create(&local_bus, &self.rng))
        }

        if self.pubkey_agg {
            info!("Setting up PublicKeyAggregationExtension");
            e3_builder =
                e3_builder.with(PublicKeyAggregatorExtension::create(&local_bus, &sortition))
        }

        if self.plaintext_agg {
            info!("Setting up PlaintextAggregationExtension (legacy)");
            e3_builder =
                e3_builder.with(PlaintextAggregatorExtension::create(&local_bus, &sortition))
        }

        if self.threshold_plaintext_agg {
            info!("Setting up ThresholdPlaintextAggregatorExtension NEW!");
            let multithread = self.ensure_multithread();
            e3_builder = e3_builder.with(ThresholdPlaintextAggregatorExtension::create(
                &local_bus,
                &sortition,
                &multithread,
            ))
        }

        if matches!(self.keyshare, Some(KeyshareKind::NonThreshold)) {
            info!("Setting up KeyshareExtension (legacy)!");
            e3_builder = e3_builder.with(KeyshareExtension::create(&local_bus, &addr, &self.cipher))
        }
        info!("building...");
        e3_builder.build().await?;

        Ok(CiphernodeHandle::new(
            addr.to_owned(),
            store,
            local_bus,
            history,
            errors,
        ))
    }

    fn ensure_multithread(&mut self) -> Addr<Multithread> {
        // If we have it cached return it
        if let Some(cached) = self.multithread_cache.clone() {
            return cached;
        }
        info!("Setting up multithread actor...");
        // Create it
        let addr = Multithread::attach(
            self.rng.clone(),
            self.cipher.clone(),
            self.threads.unwrap_or(1),
        );

        // Set the cache
        self.multithread_cache = Some(addr.clone());

        // return it
        addr
    }
}

/// Struct to cache modules required during the ciphernode construction so that providers are only
/// constructed once.
struct ProviderCaches {
    signer_cache: Option<LocalSigner<SigningKey>>,
    read_provider_cache: HashMap<ChainConfig, EthProvider<ConcreteReadProvider>>,
    write_provider_cache: HashMap<ChainConfig, EthProvider<ConcreteWriteProvider>>,
}

impl ProviderCaches {
    pub fn new() -> Self {
        ProviderCaches {
            signer_cache: None,
            read_provider_cache: HashMap::new(),
            write_provider_cache: HashMap::new(),
        }
    }

    pub async fn ensure_signer(
        &mut self,
        cipher: &Cipher,
        repositories: &Repositories,
    ) -> Result<LocalSigner<SigningKey>> {
        if let Some(ref cache) = self.signer_cache {
            return Ok(cache.clone());
        }

        let signer = load_signer_from_repository(repositories.eth_private_key(), cipher).await?;
        self.signer_cache = Some(signer.clone());
        Ok(signer)
    }

    pub async fn ensure_read_provider(
        &mut self,
        chain: &ChainConfig,
    ) -> Result<EthProvider<ConcreteReadProvider>> {
        if let Some(cache) = self.read_provider_cache.get(chain) {
            return Ok(cache.clone());
        }
        let rpc_url = chain.rpc_url()?;
        let provider_config = ProviderConfig::new(rpc_url, chain.rpc_auth.clone());
        let read_provider = provider_config.create_readonly_provider().await?;
        self.read_provider_cache
            .insert(chain.clone(), read_provider.clone());
        Ok(read_provider)
    }

    pub async fn ensure_write_provider(
        &mut self,
        repositories: &Repositories,
        chain: &ChainConfig,
        cipher: &Cipher,
    ) -> Result<EthProvider<ConcreteWriteProvider>> {
        if let Some(cache) = self.write_provider_cache.get(chain) {
            return Ok(cache.clone());
        }

        let signer = self.ensure_signer(cipher, repositories).await?;
        let rpc_url = chain.rpc_url()?;
        let provider_config = ProviderConfig::new(rpc_url, chain.rpc_auth.clone());
        let write_provider = provider_config.create_signer_provider(&signer).await?;
        self.write_provider_cache
            .insert(chain.clone(), write_provider.clone());
        Ok(write_provider)
    }
}
