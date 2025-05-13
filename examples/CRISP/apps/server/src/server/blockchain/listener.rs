use alloy::{
    primitives::{Address, B256},
    providers::{Provider, ProviderBuilder, RootProvider},
    rpc::types::{BlockNumberOrTag, Filter, Log},
    sol_types::SolEvent,
    transports::BoxTransport,
};
use eyre::eyre;
use eyre::Result;
use futures::stream::StreamExt;
use log::{error, info};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use super::events::{
    CiphertextOutputPublished, CommitteePublished, E3Activated, InputPublished,
    PlaintextOutputPublished,
};

pub trait ContractEvent: Send + Sync + 'static {
    fn process(&self, log: Log) -> Result<()>;
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
        let mut stream = self
            .provider
            .subscribe_logs(&self.filter)
            .await?
            .into_stream();
        while let Some(log) = stream.next().await {
            if let Some(topic0) = log.topic0() {
                if let Some(decoder) = self.handlers.get(topic0) {
                    match decoder(log.clone()) {
                        Ok(event) => {
                            event.process(log)?;
                        }
                        Err(e) => {
                            println!("Error decoding event 0x{:x}: {:?}", topic0, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

pub struct EnclaveContract {
    provider: Arc<RootProvider<BoxTransport>>,
}

impl EnclaveContract {
    pub async fn new(rpc_url: &str) -> Result<Self> {
        let provider = ProviderBuilder::new().on_builtin(rpc_url).await?;
        Ok(Self {
            provider: Arc::new(provider),
        })
    }

    pub fn add_listener(&self, contract_address: Address) -> EventListener {
        let filter = Filter::new()
            .address(contract_address)
            .from_block(BlockNumberOrTag::Latest);

        EventListener::new(self.provider.clone(), filter)
    }
}

pub async fn start_listener(
    rpc_url: &str,
    enclave_address: &str,
    registry_address: &str,
) -> Result<()> {
    let enclave_address: Address = enclave_address
        .parse()
        .map_err(|_| eyre!("Failed to parse enclave_address: {}", enclave_address))?;
    let registry_address: Address = registry_address
        .parse()
        .map_err(|_| eyre!("Failed to parse registry_address: {}", registry_address))?;

    loop {
        match run_listener(rpc_url, enclave_address, registry_address).await {
            Ok(_) => {
                info!("Listener finished successfully. Checking for reconnection...");
            }
            Err(e) => {
                error!(
                    "Error occurred in listener: {}. Reconnecting after delay...",
                    e
                );
            }
        }
        sleep(Duration::from_secs(5)).await;
    }
}

// Separate function to encapsulate listener logic
async fn run_listener(
    rpc_url: &str,
    enclave_address: Address,
    registry_address: Address,
) -> Result<()> {
    let manager = EnclaveContract::new(rpc_url).await?;

    let mut enclave_listener = manager.add_listener(enclave_address);
    enclave_listener.add_event_handler::<E3Activated>();
    enclave_listener.add_event_handler::<InputPublished>();
    enclave_listener.add_event_handler::<PlaintextOutputPublished>();
    enclave_listener.add_event_handler::<CiphertextOutputPublished>();

    let mut registry_listener = manager.add_listener(registry_address);
    registry_listener.add_event_handler::<CommitteePublished>();

    let enclave_handle = tokio::spawn(async move {
        match enclave_listener.listen().await {
            Ok(_) => info!("Enclave listener finished"),
            Err(e) => error!("Error in enclave listener: {}", e),
        }
    });

    let registry_handle = tokio::spawn(async move {
        match registry_listener.listen().await {
            Ok(_) => info!("Registry listener finished"),
            Err(e) => error!("Error in registry listener: {}", e),
        }
    });

    tokio::try_join!(enclave_handle, registry_handle)?;

    Ok(())
}
