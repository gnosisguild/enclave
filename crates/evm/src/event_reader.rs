// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::helpers::EthProvider;
use actix::prelude::*;
use actix::{Addr, Recipient};
use alloy::eips::BlockNumberOrTag;
use alloy::primitives::Address;
use alloy::primitives::{LogData, B256};
use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use anyhow::{anyhow, Result};
use e3_data::{AutoPersist, Persistable, Repository};
use e3_events::{
    BusError, EnclaveErrorType, EnclaveEvent, EnclaveEventData, EventBus, EventId, Subscribe,
};
use e3_events::{Event, EventManager};
use futures_util::stream::StreamExt;
use std::collections::HashSet;
use tokio::select;
use tokio::sync::oneshot;
use tracing::{error, info, instrument, trace, warn};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "()")]
pub enum EnclaveEvmEvent {
    /// Register a reader with the coordinator before it starts processing
    RegisterReader,
    /// Signal that this reader has completed historical sync
    HistoricalSyncComplete,
    /// An actual event from the blockchain
    Event {
        event: EnclaveEvent,
        block: Option<u64>,
    },
}

impl EnclaveEvmEvent {
    pub fn new(event: EnclaveEvent, block: Option<u64>) -> Self {
        Self::Event { event, block }
    }

    pub fn get_id(&self) -> EventId {
        EventId::hash(self.clone())
    }
}

pub type ExtractorFn<E> = fn(&LogData, Option<&B256>, u64) -> Option<E>;

pub struct EvmEventReaderParams<P> {
    provider: EthProvider<P>,
    extractor: ExtractorFn<EnclaveEvent>,
    contract_address: Address,
    start_block: Option<u64>,
    processor: Recipient<EnclaveEvmEvent>,
    bus: EventManager<EnclaveEvent>,
    state: Persistable<EvmEventReaderState>,
    rpc_url: String,
}

#[derive(Default, serde::Serialize, serde::Deserialize, Clone)]
pub struct EvmEventReaderState {
    pub ids: HashSet<EventId>,
    pub last_block: Option<u64>,
}

/// Connects to Enclave.sol converting EVM events to EnclaveEvents
pub struct EvmEventReader<P> {
    /// The alloy provider
    provider: Option<EthProvider<P>>,
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
    /// Processor to forward events an actor
    processor: Recipient<EnclaveEvmEvent>,
    /// Event bus for error propagation only
    bus: EventManager<EnclaveEvent>,
    /// The auto persistable state of the event reader
    state: Persistable<EvmEventReaderState>,
    /// The RPC URL for the provider
    rpc_url: String,
}

impl<P: Provider + Clone + 'static> EvmEventReader<P> {
    pub fn new(params: EvmEventReaderParams<P>) -> Self {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        Self {
            contract_address: params.contract_address,
            provider: Some(params.provider),
            extractor: params.extractor,
            shutdown_rx: Some(shutdown_rx),
            shutdown_tx: Some(shutdown_tx),
            start_block: params.start_block,
            processor: params.processor,
            bus: params.bus,
            state: params.state,
            rpc_url: params.rpc_url,
        }
    }

    pub async fn attach(
        provider: EthProvider<P>,
        extractor: ExtractorFn<EnclaveEvent>,
        contract_address: &str,
        start_block: Option<u64>,
        processor: &Recipient<EnclaveEvmEvent>,
        bus: &EventManager<EnclaveEvent>,
        repository: &Repository<EvmEventReaderState>,
        rpc_url: String,
    ) -> Result<Addr<Self>> {
        let sync_state = repository
            .clone()
            .load_or_default(EvmEventReaderState::default())
            .await?;

        let params = EvmEventReaderParams {
            provider,
            extractor,
            contract_address: contract_address.parse()?,
            start_block,
            processor: processor.clone(),
            bus: bus.clone(),
            state: sync_state,
            rpc_url,
        };

        let addr = EvmEventReader::new(params).start();

        processor.do_send(EnclaveEvmEvent::RegisterReader);

        bus.do_send(Subscribe::new("Shutdown", addr.clone().into()));
        Ok(addr)
    }
}

impl<P: Provider + Clone + 'static> Actor for EvmEventReader<P> {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let reader_addr = ctx.address();
        let bus = self.bus.clone();

        let Some(provider) = self.provider.take() else {
            error!("Could not start event reader as provider has already been used.");
            return;
        };

        let extractor = self.extractor;
        let Some(shutdown) = self.shutdown_rx.take() else {
            bus.err(EnclaveErrorType::Evm, anyhow!("shutdown already called"));
            return;
        };

        let contract_address = self.contract_address;
        let start_block = self.start_block;
        let rpc_url = self.rpc_url.clone();

        ctx.spawn(
            async move {
                stream_from_evm(
                    provider,
                    &contract_address,
                    reader_addr.clone(),
                    extractor,
                    shutdown,
                    start_block,
                    &bus,
                    rpc_url,
                )
                .await
            }
            .into_actor(self),
        );
    }
}

#[instrument(name = "evm_event_reader", skip_all)]
async fn stream_from_evm<P: Provider + Clone + 'static>(
    provider: EthProvider<P>,
    contract_address: &Address,
    reader_addr: Addr<EvmEventReader<P>>,
    extractor: fn(&LogData, Option<&B256>, u64) -> Option<EnclaveEvent>,
    mut shutdown: oneshot::Receiver<()>,
    start_block: Option<u64>,
    bus: &EventManager<EnclaveEvent>,
    rpc_url: String,
) {
    let chain_id = provider.chain_id();
    let provider_ref = provider.provider();

    if start_block.unwrap_or(0) == 0 && !is_local_node(&rpc_url) {
        error!(
            "Querying from block 0 on a non-local node ({}) without a specific start_block is not allowed.",
            rpc_url
        );
        bus.err(
            EnclaveErrorType::Evm,
            anyhow!(
                "Misconfiguration: Attempted to query historical events from genesis on a non-local node. \
                Please specify a `start_block` for contract address {contract_address} on chain {chain_id} using rpc {rpc_url}"
            )
        );
        return;
    }

    let historical_filter = Filter::new()
        .address(*contract_address)
        .from_block(start_block.unwrap_or(0));
    let current_filter = Filter::new()
        .address(*contract_address)
        .from_block(BlockNumberOrTag::Latest);

    // Historical events
    match provider_ref.get_logs(&historical_filter).await {
        Ok(historical_logs) => {
            info!("Fetched {} historical events", historical_logs.len());
            for log in historical_logs {
                let block_number = log.block_number;
                if let Some(event) = extractor(log.data(), log.topic0(), chain_id) {
                    trace!("Processing historical log");
                    reader_addr.do_send(EnclaveEvmEvent::new(event, block_number));
                }
            }

            reader_addr.do_send(EnclaveEvmEvent::HistoricalSyncComplete);
        }
        Err(e) => {
            error!("Failed to fetch historical events: {}", e);
            bus.err(EnclaveErrorType::Evm, anyhow!(e));
            return;
        }
    }

    info!("Subscribing to live events");
    match provider_ref.subscribe_logs(&current_filter).await {
        Ok(subscription) => {
            let id: B256 = subscription.local_id().clone();
            let mut stream = subscription.into_stream();

            loop {
                select! {
                    maybe_log = stream.next() => {
                        match maybe_log {
                            Some(log) => {
                                let block_number = log.block_number;
                                trace!("Received log from EVM");

                                let Some(event) = extractor(log.data(), log.topic0(), chain_id) else {
                                    trace!("Unknown log from EVM. This will happen from time to time.");
                                    continue;
                                };

                                trace!("Extracted EVM Event: {}", event);
                                reader_addr.do_send(EnclaveEvmEvent::new(event, block_number));
                            }
                            None => break, // Stream ended
                        }
                    }
                    _ = &mut shutdown => {
                        info!("Received shutdown signal, stopping EVM stream");
                        match provider_ref.unsubscribe(id).await {
                            Ok(_) => info!("Unsubscribed successfully from EVM event stream"),
                            Err(err) => error!("Cannot unsubscribe from EVM event stream: {}", err),
                        };
                        break;
                    }
                }
            }
        }
        Err(e) => {
            bus.err(EnclaveErrorType::Evm, anyhow!("{}", e));
        }
    }

    info!("Exiting stream loop");
}

fn is_local_node(rpc_url: &str) -> bool {
    rpc_url.contains("localhost") || rpc_url.contains("127.0.0.1")
}

impl<P: Provider + Clone + 'static> Handler<EnclaveEvent> for EvmEventReader<P> {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        if let EnclaveEventData::Shutdown(_) = msg.into_data() {
            if let Some(shutdown) = self.shutdown_tx.take() {
                let _ = shutdown.send(());
            }
        }
    }
}

impl<P: Provider + Clone + 'static> Handler<EnclaveEvmEvent> for EvmEventReader<P> {
    type Result = ();

    #[instrument(name = "evm_event_reader", skip_all)]
    fn handle(&mut self, msg: EnclaveEvmEvent, _: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvmEvent::RegisterReader | EnclaveEvmEvent::HistoricalSyncComplete => {
                self.processor.do_send(msg);
            }

            EnclaveEvmEvent::Event { event, block } => {
                match self.state.try_mutate(|mut state| {
                    let temp_wrapped = EnclaveEvmEvent::Event {
                        event: event.clone(),
                        block,
                    };
                    let event_id = temp_wrapped.get_id();

                    trace!("Processing event: {}", event_id);
                    trace!("Cache length: {}", state.ids.len());

                    if state.ids.contains(&event_id) {
                        warn!(
                            "Event id {} has already been seen and was not forwarded",
                            &event_id
                        );
                        return Ok(state);
                    }

                    let event_type = event.event_type();

                    self.processor.do_send(EnclaveEvmEvent::Event {
                        event: event.clone(),
                        block,
                    });

                    // Save processed IDs
                    trace!("Storing event(EVM) in cache {}({})", event_type, event_id);
                    state.ids.insert(event_id);
                    state.last_block = block;

                    Ok(state)
                }) {
                    Ok(_) => (),
                    Err(err) => self.bus.err(EnclaveErrorType::Evm, err),
                }
            }
        }
    }
}
