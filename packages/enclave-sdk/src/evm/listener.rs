use alloy::{
    primitives::{Address, B256},
    providers::{Provider, ProviderBuilder, RootProvider},
    rpc::types::{BlockNumberOrTag, Filter, Log},
    sol_types::SolEvent,
    transports::BoxTransport,
};
use eyre::Result;
use futures::stream::StreamExt;
use futures_util::future::FutureExt;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};
use tokio::sync::RwLock;

type EventHandler =
    Box<dyn Fn(&Log) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct EventListener {
    provider: Arc<RootProvider<BoxTransport>>,
    filter: Filter,
    handlers: Arc<RwLock<HashMap<B256, Vec<EventHandler>>>>,
}

impl EventListener {
    pub fn new(provider: Arc<RootProvider<BoxTransport>>, filter: Filter) -> Self {
        Self {
            provider,
            filter,
            handlers: Arc::new(RwLock::new(HashMap::new())),
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

    pub async fn listen(&self) -> Result<()> {
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

    pub async fn create_contract_listener(ws_url: &str, contract_address: &str) -> Result<Self> {
        let provider = Arc::new(ProviderBuilder::new().on_builtin(ws_url).await?);
        let address = contract_address.parse::<Address>()?;
        let filter = Filter::new()
            .address(address)
            .from_block(BlockNumberOrTag::Latest);
        Ok(EventListener::new(provider, filter))
    }
}

use std::time::{SystemTime, UNIX_EPOCH};
