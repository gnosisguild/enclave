use crate::helpers::{ReadonlyProvider, WithChainId};
use actix::prelude::*;
use actix::{Addr, Recipient};
use alloy::eips::BlockNumberOrTag;
use alloy::primitives::Address;
use alloy::primitives::{LogData, B256};
use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use alloy::transports::{BoxTransport, Transport};
use anyhow::{anyhow, Result};
use data::{AutoPersist, Persistable, Repository};
use events::{get_tag, BusError, EnclaveErrorType, EnclaveEvent, EventBus, EventId, Subscribe};
use futures_util::stream::StreamExt;
use std::collections::HashSet;
use tokio::select;
use tokio::sync::oneshot;
use tracing::{error, info, instrument, trace, warn};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "()")]
pub struct EnclaveEvmEvent {
    pub event: EnclaveEvent,
    pub block: Option<u64>,
}

impl EnclaveEvmEvent {
    pub fn new(event: EnclaveEvent, block: Option<u64>) -> Self {
        Self { event, block }
    }

    pub fn get_id(&self) -> EventId {
        EventId::hash(self.clone())
    }
}

pub type ExtractorFn<E> = fn(&LogData, Option<&B256>, u64) -> Option<E>;

pub type EventReader = EvmEventReader<ReadonlyProvider>;

pub struct EvmEventReaderParams<P, T>
where
    P: Provider<T> + Clone + 'static,
    T: Transport + Clone + Unpin,
{
    provider: WithChainId<P, T>,
    extractor: ExtractorFn<EnclaveEvent>,
    contract_address: Address,
    start_block: Option<u64>,
    bus: Addr<EventBus>,
    state: Persistable<EvmEventReaderState>,
}

#[derive(Default, serde::Serialize, serde::Deserialize, Clone)]
pub struct EvmEventReaderState {
    pub ids: HashSet<EventId>,
    pub last_block: Option<u64>,
}

/// Connects to Enclave.sol converting EVM events to EnclaveEvents
pub struct EvmEventReader<P, T = BoxTransport>
where
    P: Provider<T> + Clone + 'static,
    T: Transport + Clone + Unpin,
{
    /// The alloy provider
    provider: Option<WithChainId<P, T>>,
    /// The contract address
    contract_address: Address,
    /// The Extractor function to determine which events to extract and convert to EnclaveEvents
    extractor: ExtractorFn<EnclaveEvent>,
    /// A shutdown receiver to listen to for shutdown signals sent to the loop this is only used
    /// internally. You should send the Shutdown signal to the reader directly or via the EventBus
    shutdown_rx: Option<oneshot::Receiver<()>>,
    /// The sender for the shutdown signal this is only used internally
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// The block that processing should start from
    start_block: Option<u64>,
    /// Event bus for error propagation
    bus: Addr<EventBus>,
    /// The auto persistable state of the event reader
    state: Persistable<EvmEventReaderState>,
}

impl<P, T> EvmEventReader<P, T>
where
    P: Provider<T> + Clone + 'static,
    T: Transport + Clone + Unpin,
{
    pub fn new(params: EvmEventReaderParams<P, T>) -> Self {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        Self {
            contract_address: params.contract_address,
            provider: Some(params.provider),
            extractor: params.extractor,
            shutdown_rx: Some(shutdown_rx),
            shutdown_tx: Some(shutdown_tx),
            start_block: params.start_block,
            bus: params.bus,
            state: params.state,
        }
    }

    pub async fn attach(
        provider: &WithChainId<P, T>,
        extractor: ExtractorFn<EnclaveEvent>,
        contract_address: &str,
        start_block: Option<u64>,
        bus: &Addr<EventBus>,
        repository: &Repository<EvmEventReaderState>,
    ) -> Result<Addr<Self>> {
        let sync_state = repository
            .clone()
            .load_or_default(EvmEventReaderState::default())
            .await?;

        let params = EvmEventReaderParams {
            provider: provider.clone(),
            extractor,
            contract_address: contract_address.parse()?,
            start_block,
            bus: bus.clone(),
            state: sync_state,
        };
        let addr = EvmEventReader::new(params).start();

        bus.do_send(Subscribe::new("Shutdown", addr.clone().into()));

        Ok(addr)
    }
}

impl<P, T> Actor for EvmEventReader<P, T>
where
    P: Provider<T> + Clone + 'static,
    T: Transport + Clone + Unpin,
{
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let processor = ctx.address().recipient();
        let bus = self.bus.clone();
        let Some(provider) = self.provider.take() else {
            tracing::error!("Could not start event reader as provider has already been used.");
            return;
        };

        let extractor = self.extractor;
        let Some(shutdown) = self.shutdown_rx.take() else {
            bus.err(EnclaveErrorType::Evm, anyhow!("shutdown already called"));
            return;
        };

        let contract_address = self.contract_address;
        let start_block = self.start_block;
        let tag = get_tag();
        ctx.spawn(
            async move {
                stream_from_evm(
                    provider,
                    &contract_address,
                    &processor,
                    extractor,
                    shutdown,
                    start_block,
                    &bus,
                    &tag,
                )
                .await
            }
            .into_actor(self),
        );
    }
}

#[instrument(name = "evm_event_reader", skip_all, fields(id=id))]
async fn stream_from_evm<P: Provider<T>, T: Transport + Clone>(
    provider: WithChainId<P, T>,
    contract_address: &Address,
    processor: &Recipient<EnclaveEvmEvent>,
    extractor: fn(&LogData, Option<&B256>, u64) -> Option<EnclaveEvent>,
    mut shutdown: oneshot::Receiver<()>,
    start_block: Option<u64>,
    bus: &Addr<EventBus>,
    id: &str,
) {
    let chain_id = provider.get_chain_id();
    let provider = provider.get_provider();

    let historical_filter = Filter::new()
        .address(contract_address.clone())
        .from_block(start_block.unwrap_or(0));
    let current_filter = Filter::new()
        .address(*contract_address)
        .from_block(BlockNumberOrTag::Latest);

    // Historical events
    match provider.clone().get_logs(&historical_filter).await {
        Ok(historical_logs) => {
            info!("Fetched {} historical events", historical_logs.len());
            for log in historical_logs {
                let block_number = log.block_number;
                if let Some(event) = extractor(log.data(), log.topic0(), chain_id) {
                    trace!("Processing historical log");
                    processor.do_send(EnclaveEvmEvent::new(event, block_number));
                }
            }
        }
        Err(e) => {
            warn!("Failed to fetch historical events: {}", e);
            bus.err(EnclaveErrorType::Evm, anyhow!(e));
            return;
        }
    }

    info!("subscribing to live events");
    match provider.subscribe_logs(&current_filter).await {
        Ok(subscription) => {
            let mut stream = subscription.into_stream();
            loop {
                select! {
                    maybe_log = stream.next() => {
                        match maybe_log {
                            Some(log) => {
                                let block_number = log.block_number;
                                trace!("Received log from EVM");
                                let Some(event) = extractor(log.data(), log.topic0(), chain_id)
                                else {
                                    warn!("Failed to extract log from EVM.");
                                    continue;
                                };
                                info!("Extracted Evm Event: {}", event);
                                processor.do_send(EnclaveEvmEvent::new(event, block_number));

                            }
                            None => break, // Stream ended
                        }
                    }
                    _ = &mut shutdown => {
                        info!("Received shutdown signal, stopping EVM stream");
                        break;
                    }
                }
            }
        }
        Err(e) => {
            bus.err(EnclaveErrorType::Evm, anyhow!("{}", e));
        }
    };
    info!("Exiting stream loop");
}

impl<P, T> Handler<EnclaveEvent> for EvmEventReader<P, T>
where
    P: Provider<T> + Clone + 'static,
    T: Transport + Clone + Unpin,
{
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        if let EnclaveEvent::Shutdown { .. } = msg {
            if let Some(shutdown) = self.shutdown_tx.take() {
                let _ = shutdown.send(());
            }
        }
    }
}

impl<P, T> Handler<EnclaveEvmEvent> for EvmEventReader<P, T>
where
    P: Provider<T> + Clone + 'static,
    T: Transport + Clone + Unpin,
{
    type Result = ();

    #[instrument(name="evm_event_reader", skip_all, fields(id = get_tag()))]
    fn handle(&mut self, wrapped: EnclaveEvmEvent, _: &mut Self::Context) -> Self::Result {
        match self.state.try_mutate(|mut state| {
            let event_id = wrapped.get_id();
            info!("Processing event: {}", event_id);
            info!("cache length: {}", state.ids.len());
            if state.ids.contains(&event_id) {
                error!(
                    "Event id {} has already been seen and was not forwarded to the bus",
                    &event_id
                );
                return Ok(state);
            }

            let event_type = wrapped.event.event_type();

            // Forward everything else to the event bus
            self.bus.do_send(wrapped.event);

            // Save processed ids
            info!("Storing event(EVM) in cache {}({})", event_type, event_id);

            state.ids.insert(event_id);
            state.last_block = wrapped.block;

            Ok(state)
        }) {
            Ok(_) => (),
            Err(err) => self.bus.err(EnclaveErrorType::Evm, err),
        }
    }
}
