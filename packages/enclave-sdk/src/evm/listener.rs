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

// Define a domain event type that's decoupled from Log
pub trait DomainEvent: Send + Sync {
    fn signature(&self) -> B256;
}

pub struct EventListener {
    provider: Arc<RootProvider<BoxTransport>>,
    filter: Filter,
    handlers: HashMap<B256, Vec<Box<dyn Fn(&Log) -> Result<()> + Send + Sync>>>,
}

impl EventListener {
    pub fn new(provider: Arc<RootProvider<BoxTransport>>, filter: Filter) -> Self {
        Self {
            provider,
            filter,
            handlers: HashMap::new(),
        }
    }

    pub fn add_event_handler<E>(
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
        println!("ADDING TO HANDLER!");
        println!("Handler len before: {}", self.handlers.len());
        self.handlers
            .entry(signature)
            .or_insert_with(Vec::new)
            .push(wrapped_handler);
        println!("Handler len after: {}", self.handlers.len());
    }

    pub async fn listen(&self) -> Result<()> {
        let mut stream = self
            .provider
            .subscribe_logs(&self.filter)
            .await?
            .into_stream();

        while let Some(log) = stream.next().await {
            if let Some(topic0) = log.topic0() {
                if let Some(handlers) = self.handlers.get(topic0) {
                    for handler in handlers {
                        if let Err(e) = handler(&log) {
                            println!("Error processing event 0x{:x}: {:?}", topic0, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn create_contract_listener(
        ws_url: &str,
        contract_address: &Address,
    ) -> Result<Self> {
        let provider = Arc::new(ProviderBuilder::new().on_builtin(ws_url).await?);
        let filter = Filter::new()
            .address(contract_address.clone())
            .from_block(BlockNumberOrTag::Latest);

        Ok(EventListener::new(provider, filter))
    }
}
