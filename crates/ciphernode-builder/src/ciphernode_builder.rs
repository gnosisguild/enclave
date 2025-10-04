// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Addr};
use e3_aggregator::ext::{
    PlaintextAggregatorExtension, PublicKeyAggregatorExtension,
    ThresholdPlaintextAggregatorExtension,
};
use e3_crypto::Cipher;
use e3_data::{DataStore, InMemStore, RepositoriesFactory};
use e3_events::{EnclaveEvent, EventBus, EventBusConfig};
use e3_fhe::ext::FheExtension;
use e3_keyshare::ext::{KeyshareExtension, ThresholdKeyshareExtension};
use e3_multithread::Multithread;
use e3_request::E3Router;
use e3_sortition::{CiphernodeSelector, Sortition, SortitionRepositoryFactory};
use e3_utils::{rand_eth_addr, SharedRng};
use std::sync::Arc;
use tracing::info;

use crate::CiphernodeSimulated;

/// Build a ciphernode configuration.
pub struct CiphernodeBuilder {
    trbfv: bool,
    address: Option<String>,
    history: bool,
    logging: bool,
    errors: bool,
    pubkey_agg: bool,
    threads: Option<usize>,
    threshold_plaintext_agg: bool,
    plaintext_agg: bool,
    source_bus: Option<Addr<EventBus<EnclaveEvent>>>,
    multithread_cache: Option<Addr<Multithread>>,
    data: Option<Addr<InMemStore>>,
    rng: SharedRng,
    cipher: Arc<Cipher>,
}

impl CiphernodeBuilder {
    pub fn new(rng: SharedRng, cipher: Arc<Cipher>) -> Self {
        Self {
            address: None,
            trbfv: false,
            logging: false,
            history: false,
            errors: false,
            pubkey_agg: false,
            plaintext_agg: false,
            threshold_plaintext_agg: false,
            source_bus: None,
            data: None,
            threads: None,
            multithread_cache: None,
            rng,
            cipher,
        }
    }

    pub fn with_source_bus(mut self, bus: &Addr<EventBus<EnclaveEvent>>) -> Self {
        self.source_bus = Some(bus.clone());
        self
    }

    pub fn with_trbfv(mut self) -> Self {
        self.trbfv = true;
        self
    }

    pub fn with_data(mut self, data: Addr<InMemStore>) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_history(mut self) -> Self {
        self.history = true;
        self
    }

    pub fn with_errors(mut self) -> Self {
        self.errors = true;
        self
    }

    pub fn with_address(mut self, addr: &str) -> Self {
        self.address = Some(addr.to_owned());
        self
    }

    pub fn with_logging(mut self) -> Self {
        self.logging = true;
        self
    }

    pub fn with_pubkey_aggregation(mut self) -> Self {
        self.pubkey_agg = true;
        self
    }

    pub fn with_plaintext_aggregation(mut self) -> Self {
        self.plaintext_agg = true;
        self
    }

    pub fn with_injected_multithread(mut self, multithread: Addr<Multithread>) -> Self {
        self.multithread_cache = Some(multithread);
        self
    }

    pub fn with_threads(mut self, threads: usize) -> Self {
        self.threads = Some(threads);
        self
    }

    pub fn with_threshold_plaintext_aggregation(mut self) -> Self {
        self.threshold_plaintext_agg = true;
        self
    }

    pub async fn build(mut self) -> anyhow::Result<CiphernodeSimulated> {
        // Local bus for ciphernode events
        let local_bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();

        if let Some(ref bus) = self.source_bus {
            info!("Setting up Event pipe");
            EventBus::pipe(bus, &local_bus);
        }

        // History collector for taking historical events for analysis
        let history = if self.history {
            info!("Setting up history collector");
            Some(EventBus::<EnclaveEvent>::history(&local_bus))
        } else {
            None
        };

        let errors = if self.errors {
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

        // create data actor for saving data
        let data_actor = self
            .data
            .clone()
            .unwrap_or_else(|| InMemStore::new(self.logging).start());

        // Sortition
        let store = DataStore::from(&data_actor);
        let repositories = store.repositories();
        let sortition = Sortition::attach(&local_bus, repositories.sortition()).await?;

        // Ciphernode Selector
        CiphernodeSelector::attach(&local_bus, &sortition, &addr);

        // E3 specific setup
        let mut e3_builder = E3Router::builder(&local_bus, store);

        if self.trbfv {
            let multithread = self.ensure_multithread();
            info!("Setting up ThresholdKeyshareExtension");
            e3_builder = e3_builder.with(ThresholdKeyshareExtension::create(
                &local_bus,
                &self.cipher,
                &multithread,
                &addr,
            ))
        }

        if !self.trbfv || self.pubkey_agg || self.plaintext_agg {
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

        if !self.trbfv {
            info!("Setting up KeyshareExtension (legacy)!");
            e3_builder = e3_builder.with(KeyshareExtension::create(&local_bus, &addr, &self.cipher))
        }
        info!("building...");
        e3_builder.build().await?;

        Ok(CiphernodeSimulated::new(
            addr.to_owned(),
            data_actor.clone(),
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
