// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.;

use actix::{Actor, Addr};
use e3_aggregator::ext::{PlaintextAggregatorExtension, PublicKeyAggregatorExtension};
use e3_crypto::Cipher;
use e3_data::{DataStore, InMemStore, RepositoriesFactory};
use e3_events::{EnclaveEvent, EventBus, EventBusConfig};
use e3_fhe::{ext::FheExtension, SharedRng};
use e3_keyshare::ext::{KeyshareExtension, ThresholdKeyshareExtension};
use e3_multithread::Multithread;
use e3_request::E3Router;
use e3_sortition::{CiphernodeSelector, Sortition, SortitionRepositoryFactory};
use std::sync::Arc;

use crate::{ciphernode_system::CiphernodeSimulated, rand_eth_addr};

/// Build a ciphernode configuration.
pub struct CiphernodeBuilder {
    trbfv: bool,
    address: Option<String>,
    history: bool,
    logging: bool,
    errors: bool,
    pubkey_agg: bool,
    plaintext_agg: bool,
    source_bus: Option<Addr<EventBus<EnclaveEvent>>>,
    injected_multithread: Option<Addr<Multithread>>,
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
            source_bus: None,
            data: None,
            injected_multithread: None,
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
        self.injected_multithread = Some(multithread);
        self
    }

    pub async fn build(self) -> anyhow::Result<CiphernodeSimulated> {
        // Local bus for ciphernode events
        let local_bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();

        if let Some(bus) = self.source_bus {
            EventBus::pipe(&bus, &local_bus);
        }

        // History collector for taking historical events for analysis
        let history = if self.history {
            Some(EventBus::<EnclaveEvent>::history(&local_bus))
        } else {
            None
        };

        let errors = if self.errors {
            Some(EventBus::<EnclaveEvent>::error(&local_bus))
        } else {
            None
        };

        let addr = if let Some(addr) = self.address.clone() {
            addr
        } else {
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
            let multithread = self
                .injected_multithread
                .clone()
                .unwrap_or_else(|| Multithread::attach(self.rng.clone(), self.cipher.clone(), 1));

            e3_builder = e3_builder.with(ThresholdKeyshareExtension::create(
                &local_bus,
                &self.cipher,
                &multithread,
                &self.rng,
                &addr,
            ))
        } else {
            // NOTE: leaving this here and Keyshare below to ensure correct rng if it doesnt matter then revisit later
            e3_builder = e3_builder.with(FheExtension::create(&local_bus, &self.rng))
        }

        if self.pubkey_agg {
            e3_builder =
                e3_builder.with(PublicKeyAggregatorExtension::create(&local_bus, &sortition))
        }

        if self.plaintext_agg {
            e3_builder =
                e3_builder.with(PlaintextAggregatorExtension::create(&local_bus, &sortition))
        }

        // NOTE: keeping this in this order incase will revisit if it does not break tests
        if !self.trbfv {
            e3_builder = e3_builder.with(KeyshareExtension::create(&local_bus, &addr, &self.cipher))
        }

        e3_builder.build().await?;

        Ok(CiphernodeSimulated::new(
            addr.to_owned(),
            data_actor.clone(),
            local_bus,
            history,
            errors,
        ))
    }
}
