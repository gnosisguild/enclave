use alloy::{
    primitives::{Address, B256},
    providers::{Provider, ProviderBuilder, RootProvider},
    rpc::types::{BlockNumberOrTag, Filter, Log},
    sol_types::SolEvent,
    transports::BoxTransport,
};
use eyre::Result;
use futures::stream::StreamExt;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

// Define a domain event type that's decoupled from Log
pub trait DomainEvent: Send + Sync {
    fn signature(&self) -> B256;
}

#[derive(Clone)]
pub struct EventListener {
    provider: Arc<RootProvider<BoxTransport>>,
    filter: Filter,
    handlers: Arc<RwLock<HashMap<B256, Vec<Box<dyn Fn(&Log) -> Result<()> + Send + Sync>>>>>,
}

impl EventListener {
    pub fn new(provider: Arc<RootProvider<BoxTransport>>, filter: Filter) -> Self {
        Self {
            provider,
            filter,
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_event_handler<E>(
        &mut self,
        handler: impl Fn(&E) -> Result<()> + Send + Sync + 'static,
    ) where
        E: SolEvent + 'static,
    {
        let signature = E::SIGNATURE_HASH;
        let wrapped_handler = Box::new(move |log: &Log| -> Result<()> {
            let event = log.log_decode::<E>()?.inner.data;
            handler(&event)
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
                if let Some(handlers) = self.handlers.read().await.get(topic0) {
                    for handler in handlers {
                        if let Err(e) = handler(&log) {
                            // We don't necessarily want logging here so just printing to stderr
                            // for now. We can make this fancier later if we need to.
                            eprintln!("Error processing event 0x{:x}: {:?}", topic0, e);
                        }
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
