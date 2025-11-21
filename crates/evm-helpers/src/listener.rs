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
use tracing::{error, info};

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
}

impl EventListener {
    pub fn new(provider: Arc<dyn Provider<Ethereum>>, filter: Filter) -> Self {
        Self {
            provider,
            filter,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            block_handlers: Arc::new(RwLock::new(Vec::new())),
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

    async fn listen_once(&self) -> Result<()> {
        info!("listen_once()");
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
            info!("GOT BLOCK! {:?}", block);
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

    fn start_block_listen_loop(&self) {
        info!("start_block_listen_loop");
        let this = self.clone();
        tokio::spawn(async move { this.retry_loop(|| this.block_listen_once()).await });
    }

    fn start_listen_loop(&self) {
        info!("start_listen_loop");
        let this = self.clone();
        tokio::spawn(async move { this.retry_loop(|| this.listen_once()).await });
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
                    error!("Error occurred: {}. Retrying in 5 seconds...", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    pub fn start(&self) {
        info!("Starting event listener!");
        self.start_listen_loop();
        self.start_block_listen_loop();
    }

    pub async fn create_contract_listener(ws_url: &str, contract_address: &str) -> Result<Self> {
        let provider = Arc::new(ProviderBuilder::new().connect(ws_url).await?);
        let address = contract_address.parse::<Address>()?;
        let filter = Filter::new()
            .address(address)
            .from_block(BlockNumberOrTag::Latest);
        Ok(EventListener::new(provider, filter))
    }
}

async fn retry_with_backoff<F, Fut>(mut f: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    loop {
        match f().await {
            Ok(_) => {
                sleep(Duration::from_secs(1)).await;
            }
            Err(e) => {
                error!("Error occurred: {}. Retrying in 5 seconds...", e);
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
