// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::events::{EnclaveEvmEvent, EvmEventProcessor, EvmLog};
use crate::helpers::EthProvider;
use actix::prelude::*;
use actix::{Addr, Recipient};
use alloy::eips::BlockNumberOrTag;
use alloy::primitives::{LogData, B256};
use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use alloy_primitives::Address;
use anyhow::anyhow;
use e3_events::{BusHandle, ErrorDispatcher, Event, EventSubscriber};
use e3_events::{EType, EnclaveEvent, EnclaveEventData, EventId};
use futures_util::stream::StreamExt;
use std::collections::{HashMap, HashSet};
use tokio::select;
use tokio::sync::oneshot;
use tracing::{error, info, instrument};

pub type ExtractorFn<E> = fn(&LogData, Option<&B256>, u64) -> Option<E>;

pub struct EvmReadInterfaceParams<P> {
    provider: EthProvider<P>,
    processor: Recipient<EnclaveEvmEvent>,
    bus: BusHandle,
    filters: Filters,
}

#[derive(Default, serde::Serialize, serde::Deserialize, Clone)]
pub struct EvmReadInterfaceState {
    pub ids: HashSet<EventId>,
    pub last_block: Option<u64>,
}

#[derive(Clone, Default)]
pub struct Filters {
    historical: Filter,
    current: Filter,
}

impl Filters {
    pub fn new(addresses: Vec<Address>, start_block: Option<u64>) -> Self {
        let historical = Filter::new()
            .address(addresses.clone())
            .from_block(start_block.unwrap_or(0));
        let current = Filter::new()
            .address(addresses)
            .from_block(BlockNumberOrTag::Latest);

        Self {
            historical,
            current,
        }
    }

    pub fn from_routing_table<T>(table: &HashMap<Address, T>, start_block: Option<u64>) -> Self {
        let addresses: Vec<Address> = table.keys().cloned().collect();
        Self::new(addresses, start_block)
    }
}

/// Connects to Enclave.sol converting EVM events to EnclaveEvents
pub struct EvmReadInterface<P> {
    /// The alloy provider
    provider: Option<EthProvider<P>>,
    /// A shutdown receiver to listen to for shutdown signals sent to the loop this is only used
    /// internally. You should send the Shutdown signal to the reader directly or via the EventBus
    shutdown_rx: Option<oneshot::Receiver<()>>,
    /// The sender for the shutdown signal this is only used internally
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Processor to forward events an actor
    processor: EvmEventProcessor,
    /// Event bus for error propagation only
    bus: BusHandle,
    /// Filters to configure when to seek from
    filters: Filters,
}

impl<P: Provider + Clone + 'static> EvmReadInterface<P> {
    pub fn new(params: EvmReadInterfaceParams<P>) -> Self {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        Self {
            provider: Some(params.provider),
            shutdown_rx: Some(shutdown_rx),
            shutdown_tx: Some(shutdown_tx),
            processor: params.processor,
            bus: params.bus,
            filters: params.filters,
        }
    }

    pub fn setup(
        provider: &EthProvider<P>,
        next: &Recipient<EnclaveEvmEvent>,
        bus: &BusHandle,
        filters: Filters,
    ) -> Addr<Self> {
        let params = EvmReadInterfaceParams {
            provider: provider.clone(),
            processor: next.clone(),
            bus: bus.clone(),
            filters,
        };

        let addr = EvmReadInterface::new(params).start();

        bus.subscribe("Shutdown", addr.clone().into());
        addr
    }
}

impl<P: Provider + Clone + 'static> Actor for EvmReadInterface<P> {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // let reader_addr = ctx.address();
        let bus = self.bus.clone();
        let processor = self.processor.clone();
        let filters = self.filters.clone();

        let Some(provider) = self.provider.take() else {
            error!("Could not start event reader as provider has already been used.");
            return;
        };

        // let extractor = self.extractor;
        let Some(shutdown) = self.shutdown_rx.take() else {
            bus.err(EType::Evm, anyhow!("shutdown already called"));
            return;
        };

        ctx.spawn(
            async move { stream_from_evm(provider, processor, shutdown, &bus, filters).await }
                .into_actor(self),
        );
    }
}

// TODO: split this up into:
// 1. historical request (will finish)
// 2. current listener (run indefinitely)
#[instrument(name = "evm_interface", skip_all)]
async fn stream_from_evm<P: Provider + Clone + 'static>(
    provider: EthProvider<P>,
    processor: EvmEventProcessor,
    mut shutdown: oneshot::Receiver<()>,
    bus: &BusHandle,
    filters: Filters,
) {
    let chain_id = provider.chain_id();
    let provider_ref = provider.provider();

    // Historical events
    match provider_ref.get_logs(&filters.historical).await {
        Ok(historical_logs) => {
            info!("Fetched {} historical events", historical_logs.len());
            for log in historical_logs {
                processor.do_send(EnclaveEvmEvent::Log(EvmLog { log, chain_id }))
            }
        }
        Err(e) => {
            error!("Failed to fetch historical events: {}", e);
            bus.err(EType::Evm, anyhow!(e));
            return;
        }
    }
    processor.do_send(EnclaveEvmEvent::HistoricalSyncComplete(chain_id));

    info!("Subscribing to live events");
    match provider_ref.subscribe_logs(&filters.current).await {
        Ok(subscription) => {
            let id: B256 = subscription.local_id().clone();
            let mut stream = subscription.into_stream();

            loop {
                select! {
                    maybe_log = stream.next() => {
                        match maybe_log {
                            Some(log) => {
                                processor.do_send(EnclaveEvmEvent::Log(EvmLog { log, chain_id }))
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
            bus.err(EType::Evm, anyhow!("{}", e));
        }
    }

    info!("Exiting stream loop");
}

fn is_local_node(rpc_url: &str) -> bool {
    rpc_url.contains("localhost") || rpc_url.contains("127.0.0.1")
}

impl<P: Provider + Clone + 'static> Handler<EnclaveEvent> for EvmReadInterface<P> {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        if let EnclaveEventData::Shutdown(_) = msg.into_data() {
            if let Some(shutdown) = self.shutdown_tx.take() {
                let _ = shutdown.send(());
            }
        }
    }
}
