// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::{
    consensus::Header,
    network::Ethereum,
    primitives::{Address, B256},
    providers::{Provider, ProviderBuilder},
    rpc::types::{BlockNumberOrTag, Filter, Log},
    sol_types::SolEvent,
};
use eyre::Result;
use futures::stream::StreamExt;
use futures_util::future::FutureExt;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio::{sync::RwLock, time::sleep};
use tracing::{error, info, warn};

use crate::contracts::{EnclaveContractFactory, EnclaveReadOnlyProvider};

type EventHandler =
    Box<dyn Fn(&Log) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

type BlockHandler =
    Box<dyn Fn(&Header) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

#[derive(Clone)]
/// Listens for contract events
pub struct EventListener {
    provider: Arc<dyn Provider<Ethereum>>,
    filter: Filter,
    handlers: Arc<RwLock<HashMap<B256, Vec<EventHandler>>>>,
    block_handlers: Arc<RwLock<Vec<BlockHandler>>>,
    event_started: bool,
    block_started: bool,
}

impl EventListener {
    pub fn new(provider: Arc<dyn Provider<Ethereum>>, filter: Filter) -> Self {
        Self {
            provider,
            filter,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            block_handlers: Arc::new(RwLock::new(Vec::new())),
            event_started: false,
            block_started: false,
        }
    }

    pub async fn add_event_handler<E, F, Fut>(&mut self, handler: F)
    where
        E: SolEvent + Send + Clone + 'static,
        F: Fn(E) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let signature = E::SIGNATURE_HASH;
        let handler = Arc::new(handler);
        let wrapped_handler = Box::new(move |log: &Log| {
            let handler = Arc::clone(&handler);
            let log = log.clone();
            async move {
                let decoded = log.log_decode::<E>()?;
                let event = decoded.inner.data;
                handler(event.clone()).await
            }
            .boxed()
        });

        self.handlers
            .write()
            .await
            .entry(signature)
            .or_insert_with(Vec::new)
            .push(wrapped_handler);
    }

    pub async fn add_block_handler<F, Fut>(&mut self, handler: F)
    where
        F: Fn(&Header) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        info!("add_block_handler");
        self.block_handlers
            .write()
            .await
            .push(Box::new(move |h: &Header| Box::pin(handler(h))));
    }

    async fn event_listen_once(&self) -> Result<()> {
        info!("event_listen_once()");
        let mut stream = self
            .provider
            .subscribe_logs(&self.filter)
            .await?
            .into_stream();
        while let Some(log) = stream.next().await {
            if let Some(topic0) = log.topic0() {
                let topic_val = *topic0;
                if let Some(handlers) = self.handlers.read().await.get(topic0) {
                    for handler in handlers {
                        let log_clone = log.clone();
                        let fut = handler(&log_clone);
                        tokio::spawn(async move {
                            // Spawn the future so that the handlers are processed concurrently
                            if let Err(e) = fut.await {
                                eprintln!("Error processing event 0x{:x}: {:?}", topic_val, e);
                            }
                        });
                    }
                }
            }
        }
        Ok(())
    }

    async fn block_listen_once(&self) -> Result<()> {
        info!("block_listen_once()");
        let mut stream = self.provider.subscribe_blocks().await?.into_stream();
        while let Some(block) = stream.next().await {
            let handlers = self.block_handlers.read().await;
            for handler in handlers.iter() {
                let fut = handler(&block);
                tokio::spawn(async move {
                    if let Err(e) = fut.await {
                        eprintln!("Error processing block: {:?}", e);
                    }
                });
            }
        }
        Ok(())
    }

    fn ensure_block_listen_loop(&mut self) {
        info!("start_block_listen_loop");
        self.block_started = true;
        let this = self.clone();
        tokio::spawn(async move {
            let len = { this.block_handlers.read().await.len() };

            if len > 0 {
                this.retry_loop(|| this.block_listen_once()).await;
            }
        });
    }

    fn ensure_event_listen_loop(&mut self) {
        info!("ensure_event_listen_loop");
        self.event_started = true;
        let this = self.clone();
        tokio::spawn(async move {
            let len = { this.handlers.read().await.len() };
            if len > 0 {
                this.retry_loop(|| this.event_listen_once()).await;
            }
        });
    }

    async fn retry_loop<F, Fut, E>(&self, mut operation: F)
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<(), E>>,
        E: std::fmt::Display,
    {
        loop {
            match operation().await {
                Ok(_) => {
                    sleep(Duration::from_secs(1)).await;
                }
                Err(e) => {
                    error!("\n**********************************************************");
                    error!("Error occurred: {}. Retrying in 5 seconds...", e);
                    error!("**********************************************************\n\n");
                    sleep(Duration::from_secs(5)).await;
                }
            }
            warn!("Ongoing operation finished unexpectedly");
        }
    }

    pub fn start(&mut self) {
        self.ensure_event_listen_loop();
        self.ensure_block_listen_loop();
    }

    pub async fn create_contract_listener(ws_url: &str, contract_address: &str) -> Result<Self> {
        let provider = Arc::new(ProviderBuilder::new().connect(ws_url).await?);
        EventListener::create_contract_listener_from_provider(contract_address, provider)
    }

    pub fn create_contract_listener_from_provider(
        contract_address: &str,
        provider: Arc<EnclaveReadOnlyProvider>,
    ) -> Result<Self> {
        let address = contract_address.parse::<Address>()?;
        let filter = Filter::new()
            .address(address)
            .from_block(BlockNumberOrTag::Latest);
        Ok(EventListener::new(provider, filter))
    }
}
