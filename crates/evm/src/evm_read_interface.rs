// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::events::{EnclaveEvmEvent, EvmEventProcessor};
use crate::helpers::EthProvider;
use crate::log_fetcher::{backfill_to_head, fetch_logs_chunked, process_log, TimestampTracker};
use crate::HistoricalSyncComplete;
use actix::prelude::*;
use actix::{Addr, Recipient};
use alloy::eips::BlockNumberOrTag;
use alloy::primitives::{LogData, B256};
use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use alloy_primitives::Address;
use anyhow::anyhow;
use e3_events::{BusHandle, ErrorDispatcher, Event, EventSubscriber, EventType};
use e3_events::{EType, EnclaveEvent, EnclaveEventData, EventId};
use e3_utils::MAILBOX_LIMIT;
use futures_util::stream::StreamExt;
use std::collections::{HashMap, HashSet};
use tokio::select;
use tokio::sync::oneshot;
use tracing::{error, info, instrument, warn};

const MAX_RECONNECT_DELAY_SECS: u64 = 60;

pub type ExtractorFn<E> = fn(&LogData, Option<&B256>, u64) -> Option<E>;

pub struct EvmReadInterfaceParams<P> {
    provider: EthProvider<P>,
    next: Recipient<EnclaveEvmEvent>,
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
    start_block: u64,
}

impl Filters {
    pub fn new(addresses: Vec<Address>, start_block: u64) -> Self {
        let historical = Filter::new()
            .address(addresses.clone())
            .from_block(start_block);
        let current = Filter::new()
            .address(addresses)
            .from_block(BlockNumberOrTag::Latest);

        Self {
            historical,
            current,
            start_block,
        }
    }

    pub fn from_routing_table<T>(table: &HashMap<Address, T>, start_block: u64) -> Self {
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
    /// Processor to forward events
    next: EvmEventProcessor,
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
            next: params.next,
            bus: params.bus,
            filters: params.filters,
        }
    }

    pub fn setup(
        provider: &EthProvider<P>,
        next: impl Into<EvmEventProcessor>,
        bus: &BusHandle,
        filters: Filters,
    ) -> Addr<Self> {
        let params = EvmReadInterfaceParams {
            provider: provider.clone(),
            next: next.into(),
            bus: bus.clone(),
            filters,
        };

        let addr = EvmReadInterface::new(params).start();

        bus.subscribe(EventType::Shutdown, addr.clone().into());
        addr
    }
}

impl<P: Provider + Clone + 'static> Actor for EvmReadInterface<P> {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);

        let bus = self.bus.clone();
        let next = self.next.clone();
        let filters = self.filters.clone();

        let Some(provider) = self.provider.take() else {
            error!("Could not start event reader as provider has already been used.");
            return;
        };

        let Some(shutdown) = self.shutdown_rx.take() else {
            bus.err(EType::Evm, anyhow!("shutdown already called"));
            return;
        };

        ctx.spawn(
            async move { stream_from_evm(provider, next, shutdown, &bus, filters).await }
                .into_actor(self),
        );
    }
}

#[instrument(name = "evm_interface", skip_all)]
async fn stream_from_evm<P: Provider + Clone + 'static>(
    provider: EthProvider<P>,
    next: EvmEventProcessor,
    mut shutdown: oneshot::Receiver<()>,
    bus: &BusHandle,
    filters: Filters,
) {
    let chain_id = provider.chain_id();
    let provider_ref = provider.provider();
    let mut timestamp_tracker = TimestampTracker::new();

    // Determine chain head for historical fetch
    let latest_block = match provider_ref.get_block_number().await {
        Ok(bn) => bn,
        Err(e) => {
            error!(chain_id, error = %e, "Failed to get latest block number");
            bus.err(EType::Evm, anyhow!(e));
            return;
        }
    };

    // Historical events — chunked to respect RPC block-range limits
    let last_id = match fetch_logs_chunked(
        provider_ref,
        &filters.historical,
        filters.start_block,
        latest_block,
        chain_id,
        &next,
        &mut timestamp_tracker,
    )
    .await
    {
        Ok(id) => {
            info!(chain_id, "Historical sync succeeded");
            id
        }
        Err(e) => {
            error!(chain_id, error = %e, "Failed to fetch historical events — node cannot operate without full state, exiting");
            bus.err(EType::Evm, anyhow!(e));
            return;
        }
    };

    next.do_send(EnclaveEvmEvent::HistoricalSyncComplete(
        HistoricalSyncComplete::new(chain_id, last_id),
    ));

    // Live subscription with gap-fill on connect/reconnect
    subscribe_live_events(
        provider_ref,
        &next,
        &mut shutdown,
        bus,
        &filters.current,
        chain_id,
        &mut timestamp_tracker,
        latest_block,
    )
    .await;
}

async fn subscribe_live_events<P: Provider + Clone + 'static>(
    provider: &P,
    next: &EvmEventProcessor,
    shutdown: &mut oneshot::Receiver<()>,
    bus: &BusHandle,
    filter: &Filter,
    chain_id: u64,
    timestamp_tracker: &mut TimestampTracker,
    mut last_block: u64,
) {
    let mut reconnect_attempt: u32 = 0;

    loop {
        if reconnect_attempt > 0 {
            let delay_secs = (2u64.pow(reconnect_attempt.min(6))).min(MAX_RECONNECT_DELAY_SECS);
            warn!(
                chain_id,
                reconnect_attempt, delay_secs, "Reconnecting to live event stream"
            );
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
        }

        if shutdown.try_recv().is_ok() {
            info!("Shutdown signal received, stopping EVM stream");
            return;
        }

        // Backfill any blocks missed since last_block. This handles:
        //   - Blocks mined between historical fetch completion and first subscription
        //   - Blocks mined during reconnection downtime
        //   - Geth's eth_subscribe silently ignoring fromBlock
        match backfill_to_head(
            provider,
            filter,
            chain_id,
            next,
            timestamp_tracker,
            &mut last_block,
        )
        .await
        {
            Ok(_) => {}
            Err(e) => {
                warn!(chain_id, error = %e, "Gap backfill failed, will retry");
                reconnect_attempt += 1;
                continue;
            }
        }

        match provider.subscribe_logs(filter).await {
            Ok(subscription) => {
                let sub_id: B256 = subscription.local_id().clone();
                let mut stream = subscription.into_stream();
                reconnect_attempt = 0;
                info!(chain_id, "Live event subscription active");

                loop {
                    select! {
                        maybe_log = stream.next() => {
                            match maybe_log {
                                Some(log) => {
                                    if let Some(bn) = log.block_number {
                                        last_block = last_block.max(bn);
                                    }
                                    process_log(provider, log, chain_id, next, timestamp_tracker).await;
                                }
                                None => {
                                    warn!(chain_id, "Live event stream ended, will reconnect");
                                    break;
                                }
                            }
                        }
                        _ = &mut *shutdown => {
                            info!("Shutdown signal received, stopping EVM stream");
                            match provider.unsubscribe(sub_id).await {
                                Ok(_) => info!("Unsubscribed successfully from EVM event stream"),
                                Err(err) => error!(chain_id, error = %err, "Cannot unsubscribe from EVM event stream"),
                            };
                            return;
                        }
                    }
                }

                reconnect_attempt += 1;
            }
            Err(e) => {
                error!(chain_id, reconnect_attempt, error = %e, "Failed to subscribe to live events");
                bus.err(EType::Evm, anyhow!("{}", e));
                reconnect_attempt += 1;
            }
        }
    }
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
