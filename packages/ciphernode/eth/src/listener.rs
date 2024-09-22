use alloy::{
    primitives::B256,
    providers::{Provider, RootProvider},
    rpc::types::{Filter, Log},
    sol_types::SolEvent,
    transports::BoxTransport,
};
use eyre::Result;
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::marker::PhantomData;
use tokio::sync::mpsc::{Receiver, Sender};

pub trait ContractEvent: Send + Sync + 'static {
    fn process(&self, cmd_rx: Sender<Vec<u8>>) -> Result<()>;
}

impl<T> ContractEvent for T
where
    T: SolEvent + Debug + Send + Sync + 'static,
{
    fn process(&self, cmd_rx: Sender<Vec<u8>>) -> Result<()> {
        println!("Processing event: {:?}", self);
        // bus.do_send(EnclaveEvent::from(self));
        Ok(())
    }
}

pub struct EventListener {
    provider: Arc<RootProvider<BoxTransport>>,
    filter: Filter,
    handlers: HashMap<B256, Arc<dyn Fn(Log) -> Result<Box<dyn ContractEvent>> + Send + Sync>>,
    evt_tx: Sender<Vec<u8>>,
}

impl EventListener {
    pub fn new(
        provider: Arc<RootProvider<BoxTransport>>,
        filter: Filter,
        sender: Sender<Vec<u8>>,
    ) -> Self {
        Self {
            provider,
            filter,
            handlers: HashMap::new(),
            evt_tx: sender,
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
        let mut stream = self
            .provider
            .subscribe_logs(&self.filter)
            .await?
            .into_stream();
        while let Some(log) = stream.next().await {
            if let Some(topic0) = log.topic0() {
                if let Some(decoder) = self.handlers.get(topic0) {
                    if let Ok(event) = decoder(log.clone()) {
                        event.process(self.evt_tx.clone())?;
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn start_listening(&self) {
        // let listener = EventListener {
        //     self.provider,
        //     self.filter,
        //     self.handlers,
        //     self.evt_tx,
        // };
        if let Err(err) = self.listen().await {
            eprintln!("Error listening for events: {:?}", err);
        }
    }
}