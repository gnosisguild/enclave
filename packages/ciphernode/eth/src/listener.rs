use alloy::{
    primitives::B256,
    providers::{Provider, RootProvider},
    rpc::types::{Filter, Log},
     sol_types::SolEvent, transports::BoxTransport,
};
use eyre::Result;
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::fmt::Debug;

pub trait ContractEvent: Send + Sync + 'static {
    fn process(&self) -> Result<()>;
}

impl<T> ContractEvent for T
where
    T: SolEvent + Debug + Send + Sync + 'static,
{
    fn process(&self) -> Result<()> {
        println!("Processing event: {:?}", self);
        Ok(())
    }
}

pub struct EventListener {
    provider: Arc<RootProvider<BoxTransport>>,
    filter: Filter,
    handlers: HashMap<B256, Arc<dyn Fn(Log) -> Result<Box<dyn ContractEvent>> + Send + Sync>>,
}

impl EventListener {
    pub fn new(provider: Arc<RootProvider<BoxTransport>>, filter: Filter) -> Self {
        Self {
            provider,
            filter,
            handlers: HashMap::new(),
        }
    }

    pub fn add_event_handler<E>(&mut self)
    where
        E: SolEvent + ContractEvent + 'static,
    {
        let signature = E::SIGNATURE_HASH;
        let handler = Arc::new(move |log: Log| -> Result<Box<dyn ContractEvent>> {
            let event = log.log_decode::<E>()?.inner.data;
            Ok(Box::new(event))
        });

        self.handlers.insert(signature, handler);
    }

    pub async fn listen(&self) -> Result<()> {
        let sub = self.provider.subscribe_logs(&self.filter).await?;
        let mut stream = sub.into_stream();

        while let Some(log) = stream.next().await {
            if let Some(topic) = log.topic0() {
                if let Some(handler) = self.handlers.get(topic) {
                    match handler(log.clone()) {
                        Ok(event) => {
                            if let Err(err) = event.process() {
                                eprintln!("Error processing event: {:?}", err);
                            }
                        }
                        Err(err) => {
                            eprintln!("Error decoding log: {:?}", err);
                        }
                    }
                } else {
                    eprintln!("No handler found for topic: {:?}", topic);
                }
            }
        }

        Ok(())
    }
}

