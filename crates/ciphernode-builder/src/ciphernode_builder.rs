// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{CiphernodeHandle, EventSystem, EvmSystemChainBuilder, ProviderCache, WriteEnabled};
use actix::{Actor, Addr};
use anyhow::Result;
use derivative::Derivative;
use e3_aggregator::ext::{PublicKeyAggregatorExtension, ThresholdPlaintextAggregatorExtension};
use e3_aggregator::CommitteeFinalizer;
use e3_config::chain_config::ChainConfig;
use e3_crypto::Cipher;
use e3_data::{InMemStore, RepositoriesFactory};
use e3_events::{
    AggregateConfig, AggregateId, BusHandle, EnclaveEvent, EventBus, EventBusConfig, EvmEventConfig,
};
use e3_evm::{BondingRegistrySolReader, CiphernodeRegistrySolReader, EnclaveSolWriter};
use e3_evm::{CiphernodeRegistrySol, EnclaveSolReader};
use e3_fhe::ext::FheExtension;
use e3_keyshare::ext::ThresholdKeyshareExtension;
use e3_multithread::{Multithread, MultithreadReport, TaskPool};
use e3_net::{setup_with_interface, NetRepositoryFactory};
use e3_request::E3Router;
use e3_sortition::{
    CiphernodeSelector, CiphernodeSelectorFactory, FinalizedCommitteesRepositoryFactory,
    NodeStateRepositoryFactory, Sortition, SortitionBackend, SortitionRepositoryFactory,
};
use e3_sync::sync;
use e3_utils::{rand_eth_addr, SharedRng};
use std::time::Duration;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tracing::{error, info};

#[derive(Clone, Debug)]
enum EventSystemType {
    Persisted { log_path: PathBuf, kv_path: PathBuf },
    InMem,
}

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
    event_system: EventSystemType,
    in_mem_store: Option<Addr<InMemStore>>,
    keyshare: Option<KeyshareKind>,
    logging: bool,
    multithread_cache: Option<Addr<Multithread>>,
    multithread_concurrent_jobs: Option<usize>,
    multithread_report: Option<Addr<MultithreadReport>>,
    name: String,
    pubkey_agg: bool,
    rng: SharedRng,
    sortition_backend: SortitionBackend,
    source_bus: Option<BusMode<Addr<EventBus<EnclaveEvent>>>>,
    testmode_errors: bool,
    testmode_history: bool,
    task_pool: Option<TaskPool>,
    threads: Option<usize>,
    threshold_plaintext_agg: bool,
    net_config: Option<NetConfig>,
    start_buffer: bool,
}

// Simple Net Configuration
#[derive(Debug)]
struct NetConfig {
    pub peers: Vec<String>,
    pub quic_port: u16,
}

impl NetConfig {
    pub fn new(peers: Vec<String>, quic_port: u16) -> Self {
        Self { peers, quic_port }
    }
}

#[derive(Default, Debug)]
pub struct ContractComponents {
    enclave_reader: bool,
    enclave: bool,
    ciphernode_registry: bool,
    bonding_registry: bool,
}

#[derive(Clone, Debug)]
pub enum BusMode<T> {
    Forked(T),
    Source(T),
}

#[derive(Clone, Debug)]
pub enum KeyshareKind {
    Threshold,
}

impl CiphernodeBuilder {
    /// Create a new ciphernode builder.
    ///
    /// - name - Unique name for the ciphernode
    /// - rng - Arc Mutex wrapped random number generator
    /// - cipher - Cipher for encryption and decryption of sensitive data
    pub fn new(name: &str, rng: SharedRng, cipher: Arc<Cipher>) -> Self {
        Self {
            address: None,
            chains: vec![],
            cipher,
            contract_components: ContractComponents::default(),
            event_system: EventSystemType::InMem,
            in_mem_store: None,
            keyshare: None,
            logging: false,
            multithread_cache: None,
            multithread_concurrent_jobs: None,
            multithread_report: None,
            name: name.to_owned(),
            pubkey_agg: false,
            rng,
            sortition_backend: SortitionBackend::score(),
            source_bus: None,
            testmode_errors: false,
            testmode_history: false,
            task_pool: None,
            threads: None,
            threshold_plaintext_agg: false,
            net_config: None,
            start_buffer: false,
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

    /// Use the given in-mem datastore. This is useful for injecting a store dump.
    pub fn with_in_mem_datastore(mut self, store: &Addr<InMemStore>) -> Self {
        self.in_mem_store = Some(store.to_owned());
        self
    }

    /// Add persistence information for storing events and data. Without persistence information
    /// the node will run in memory by default.
    pub fn with_persistence(mut self, log_path: &PathBuf, kv_path: &PathBuf) -> Self {
        self.event_system = EventSystemType::Persisted {
            log_path: log_path.to_owned(),
            kv_path: kv_path.to_owned(),
        };
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
    /// Ensure SnapshotBuffer starts immediately instead of waiting for SyncEnded. This is important
    /// for tests that don't specifically
    pub fn testmode_start_buffer_immediately(mut self) -> Self {
        self.start_buffer = true;
        self
    }

    /// Use the node configuration on these specific chains. This will overwrite any previously
    /// given chains.
    pub fn with_chains(mut self, chains: &[ChainConfig]) -> Self {
        self.chains = chains.to_vec();
        self
    }

    /// Use the given Address to represent the node. This should be unique.
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

    /// Connect rayon work to the given threadpool
    pub fn with_shared_taskpool(mut self, pool: &TaskPool) -> Self {
        self.task_pool = Some(pool.clone());
        self
    }

    /// Shared MultithreadReport for benchmarking
    pub fn with_shared_multithread_report(mut self, report: &Addr<MultithreadReport>) -> Self {
        self.multithread_report = Some(report.clone());
        self
    }

    /// Setup how many threads to use within the multithread actor for it's rayon based workload
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.threads = Some(threads);
        self
    }

    /// This will provide one thread for the actor model and use all other threads for
    /// rayon based workloads
    pub fn with_max_threads(mut self) -> Self {
        self.threads = Some(Multithread::get_max_threads_minus(1));
        self
    }

    /// This will save the given number of threads from being used by the rayon threadpool
    pub fn with_max_threads_minus(mut self, threads: usize) -> Self {
        self.threads = Some(Multithread::get_max_threads_minus(threads));
        self
    }

    /// Set the number of concurrent jobs defaults to 1
    pub fn with_multithread_concurrent_jobs(mut self, jobs: usize) -> Self {
        self.multithread_concurrent_jobs = if jobs >= 1 { Some(jobs) } else { None };
        self
    }

    /// Setup a ThresholdPlaintextAggregator
    pub fn with_threshold_plaintext_aggregation(mut self) -> Self {
        self.threshold_plaintext_agg = true;
        self
    }

    /// Use score-based sortition (recommended)
    pub fn with_sortition_score(mut self) -> Self {
        self.sortition_backend = SortitionBackend::score();
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

    /// Setup a writable BondingRegistry for every evm chain provided
    pub fn with_contract_bonding_registry(mut self) -> Self {
        self.contract_components.bonding_registry = true;
        self
    }
    /// Setup a CiphernodeRegistry listener for every evm chain provided
    pub fn with_contract_ciphernode_registry(mut self) -> Self {
        self.contract_components.ciphernode_registry = true;
        self
    }

    /// Setup net package components.
    pub fn with_net(mut self, peers: Vec<String>, quic_port: u16) -> Self {
        self.net_config = Some(NetConfig::new(peers, quic_port));
        self
    }

    fn create_local_bus() -> Addr<EventBus<EnclaveEvent>> {
        EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start()
    }

    /// Create aggregate configuration from configured chains
    async fn create_aggregate_config(
        &self,
        provider_cache: &mut ProviderCache,
    ) -> Result<AggregateConfig> {
        let mut chain_providers = Vec::new();
        for chain in &self.chains {
            let provider = provider_cache.ensure_read_provider(chain).await?;
            chain_providers.push((chain.clone(), provider.chain_id()));
        }

        let delays = create_aggregate_delays(&chain_providers)?;
        Ok(AggregateConfig::new(delays))
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

        // Create provider cache early to use for chain validation
        let mut provider_cache = ProviderCache::new();
        let aggregate_config = self.create_aggregate_config(&mut provider_cache).await?;

        // Get an event system instance.
        let event_system =
            if let EventSystemType::Persisted { kv_path, log_path } = self.event_system.clone() {
                EventSystem::persisted(&addr, log_path, kv_path)
                    .with_event_bus(local_bus)
                    .with_aggregate_config(aggregate_config.clone())
            } else {
                if let Some(ref store) = self.in_mem_store {
                    EventSystem::in_mem_from_store(&addr, store)
                        .with_event_bus(local_bus)
                        .with_aggregate_config(aggregate_config.clone())
                } else {
                    EventSystem::in_mem(&addr)
                        .with_event_bus(local_bus)
                        .with_aggregate_config(aggregate_config.clone())
                }
            };

        let bus = event_system.handle()?;
        let store = event_system.store()?;
        let eventstore_ts = event_system.eventstore_getter_ts()?;
        let eventstore_seq = event_system.eventstore_getter_seq()?;
        let cipher = &self.cipher;
        let repositories = Arc::new(store.repositories());

        // Now we add write support as store depends on event system
        let mut provider_cache =
            provider_cache.with_write_support(Arc::clone(cipher), Arc::clone(&repositories));

        // Use the configured backend directly
        let default_backend = self.sortition_backend.clone();

        let ciphernode_selector =
            CiphernodeSelector::attach(&bus, repositories.ciphernode_selector(), &addr).await?;

        let sortition = Sortition::attach(
            &bus,
            repositories.sortition(),
            repositories.node_state(),
            repositories.finalized_committees(),
            default_backend,
            ciphernode_selector,
            &addr,
        )
        .await?;

        // Setup evm system
        // TODO: gather an async handle from the event readers in thre following function
        // that closes when they shutdown and join it with the network manager joinhandle externally
        let evm_config = setup_evm_system(
            &self.chains,
            &mut provider_cache,
            &bus,
            &self.contract_components,
            self.pubkey_agg,
        )
        .await?;

        // E3 specific setup
        let mut e3_builder = E3Router::builder(&bus, store.clone());

        if let Some(KeyshareKind::Threshold) = self.keyshare {
            let _ = self.ensure_multithread(&bus);
            let share_encryption_params = e3_trbfv::helpers::get_share_encryption_params();
            info!("Setting up ThresholdKeyshareExtension");
            e3_builder = e3_builder.with(ThresholdKeyshareExtension::create(
                &bus,
                &self.cipher,
                &addr,
                share_encryption_params,
            ))
        }

        if self.pubkey_agg {
            info!("Setting up FheExtension");
            e3_builder = e3_builder.with(FheExtension::create(&bus, &self.rng))
        }

        if self.pubkey_agg {
            info!("Setting up PublicKeyAggregationExtension");
            e3_builder = e3_builder.with(PublicKeyAggregatorExtension::create(&bus))
        }

        if self.threshold_plaintext_agg {
            info!("Setting up ThresholdPlaintextAggregatorExtension");
            let _ = self.ensure_multithread(&bus);
            e3_builder = e3_builder.with(ThresholdPlaintextAggregatorExtension::create(
                &bus, &sortition,
            ))
        }

        info!("building...");

        e3_builder.build().await?;

        let (join_handle, peer_id) = if let Some(net_config) = self.net_config {
            let repositories = store.repositories();
            setup_with_interface(
                bus.clone(),
                net_config.peers,
                &self.cipher,
                net_config.quic_port,
                repositories.libp2p_keypair(),
                eventstore_ts,
            )
            .await?
        } else {
            (
                tokio::spawn(std::future::ready(Ok(()))),
                "-not set-".to_string(),
            )
        };

        // Run the sync routine
        sync(
            &bus,
            &evm_config,
            &repositories,
            &aggregate_config,
            &eventstore_seq,
        )
        .await?;

        Ok(CiphernodeHandle::new(
            addr.to_owned(),
            store,
            bus,
            history,
            errors,
            peer_id,
            join_handle,
        ))
    }

    fn ensure_multithread(&mut self, bus: &BusHandle) -> Addr<Multithread> {
        // If we have it cached return it
        if let Some(cached) = self.multithread_cache.clone() {
            return cached;
        }

        info!("Setting up multithread actor...");

        // Setup threadpool if not set
        let task_pool = self.task_pool.clone().unwrap_or_else(|| {
            Multithread::create_taskpool(
                self.threads.unwrap_or(1),
                self.multithread_concurrent_jobs.unwrap_or(1),
            )
        });

        // Create it
        let addr = Multithread::attach(
            bus,
            self.rng.clone(),
            self.cipher.clone(),
            task_pool,
            self.multithread_report.clone(),
        );

        // Set the cache
        self.multithread_cache = Some(addr.clone());

        // return it
        addr
    }
}

/// Validate chain ID matches expected configuration
fn validate_chain_id(chain: &ChainConfig, actual_chain_id: u64) -> Result<()> {
    if let Some(expected_chain_id) = chain.chain_id {
        if actual_chain_id != expected_chain_id {
            return Err(anyhow::anyhow!(
                "Chain '{}' validation failed: expected chain_id {}, but provider returned chain_id {}",
                chain.name, expected_chain_id, actual_chain_id
            ));
        }
    }
    Ok(())
}

/// Build delay configuration for a specific chain
fn create_aggregate_delay(chain: &ChainConfig, actual_chain_id: u64) -> (AggregateId, Duration) {
    let aggregate_id = AggregateId::from_chain_id(Some(actual_chain_id));
    let finalization_ms = chain.finalization_ms.unwrap_or(0);
    let delay_us = finalization_ms * 1000; // ms → microseconds
    (aggregate_id, Duration::from_micros(delay_us))
}

/// Build delays configuration from chain providers
fn create_aggregate_delays(
    chain_providers: &[(ChainConfig, u64)],
) -> Result<HashMap<AggregateId, Duration>> {
    let mut delays = HashMap::new();

    for (chain, actual_chain_id) in chain_providers.into_iter().cloned() {
        // Validate chain_id if specified in configuration
        validate_chain_id(&chain, actual_chain_id)?;

        // Add delay if configured
        let (aggregate_id, delay_us) = create_aggregate_delay(&chain, actual_chain_id);
        delays.insert(aggregate_id, delay_us);
    }

    Ok(delays)
}

async fn setup_evm_system(
    chains: &Vec<ChainConfig>,
    provider_cache: &mut ProviderCache<WriteEnabled>,
    bus: &BusHandle,
    contract_components: &ContractComponents,
    pubkey_agg: bool,
) -> Result<EvmEventConfig> {
    let mut evm_config = EvmEventConfig::new();
    for chain in chains.iter().filter(|chain| chain.enabled.unwrap_or(true)) {
        let provider = provider_cache.ensure_read_provider(chain).await?;
        let chain_id = provider.chain_id();
        evm_config.insert(chain_id, chain.try_into()?);
        let mut system = EvmSystemChainBuilder::new(&bus, &provider);

        if contract_components.enclave {
            let write_provider = provider_cache.ensure_write_provider(chain).await?;
            let contract = &chain.contracts.enclave;
            EnclaveSolWriter::attach(&bus, write_provider.clone(), contract.address()?);
            system.with_contract(contract.address()?, move |next| {
                EnclaveSolReader::setup(&next).recipient()
            });
        }

        if contract_components.enclave_reader {
            let contract = &chain.contracts.enclave;

            system.with_contract(contract.address()?, move |next| {
                EnclaveSolReader::setup(&next).recipient()
            });
        }

        if contract_components.bonding_registry {
            let contract = &chain.contracts.bonding_registry;
            system.with_contract(contract.address()?, move |next| {
                BondingRegistrySolReader::setup(&next).recipient()
            });
        }

        if contract_components.ciphernode_registry {
            let contract = &chain.contracts.ciphernode_registry;

            system.with_contract(contract.address()?, move |next| {
                CiphernodeRegistrySolReader::setup(&next).recipient()
            });

            // TODO: Should we not let this pass and just use '?'?
            // Above if we include enclave in the config and we don't have a wallet it will fail
            match provider_cache
                    .ensure_write_provider(&chain)
                    .await
                {
                    Ok(write_provider) => {
                        CiphernodeRegistrySol::attach_writer(
                            &bus,
                            write_provider.clone(),
                            contract.address()?,
                            pubkey_agg,
                        );
                        info!("CiphernodeRegistrySolWriter attached for publishing committees");

                        if pubkey_agg {
                            info!("Attaching CommitteeFinalizer for score sortition");
                            CommitteeFinalizer::attach(&bus);
                        }
                    }
                    Err(e) => error!(
                        "Failed to create write provider (likely no wallet configured), skipping writer attachment: {}",
                        e
                    )
                }
        }
        system.build();
    }

    Ok(evm_config)
}
