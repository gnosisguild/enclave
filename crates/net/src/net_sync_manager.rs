// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Addr, AsyncContext, Handler, Message, Recipient, ResponseFuture};
use anyhow::{bail, Context, Result};
use e3_events::{
    prelude::*, trap, trap_fut, AggregateId, BusHandle, CorrelationId, EType, EnclaveEvent,
    EnclaveEventData, EventSource, EventStoreQueryBy, EventStoreQueryResponse, EventType,
    HistoricalNetSyncEventsReceived, HistoricalNetSyncStart, NetReady, TsAgg, TypedEvent,
    Unsequenced,
};
use e3_utils::MAILBOX_LIMIT;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::TryInto, sync::Arc, time::Duration};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};

use crate::{
    direct_requester::DirectRequester,
    direct_responder::DirectResponder,
    events::{await_event, IncomingRequest, NetCommand, NetEvent, PeerTarget},
    net_event_batch::{fetch_all_batched_events, BatchCursor, EventBatch, FetchEventsSince},
};

/// Maximum time to wait for a `ConnectionEstablished` event after all dials
/// failed before publishing `NetReady` anyway.
const NET_READY_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum time to wait for the `AllPeersDialed` event before giving up.
const ALL_PEERS_DIALED_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum time to wait for a peer to reconnect after sync fetch fails.
/// On restart, peers may briefly connect then disconnect (the remote side still
/// holds the old connection). Kademlia rediscovery can take up to ~120s.
const SYNC_RECONNECT_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponseValue {
    pub events: Vec<EnclaveEvent<Unsequenced>>,
    pub ts: u128,
}

impl TryInto<Vec<u8>> for SyncResponseValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        bincode::serialize(&self).context("failed to serialize sync response")
    }
}

impl TryFrom<Vec<u8>> for SyncResponseValue {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        bincode::deserialize(&value).context("failed to deserialize sync response")
    }
}

#[derive(Debug, Clone)]
pub struct SyncRequestSucceeded {
    pub response: SyncResponseValue,
}

pub struct NetSyncManager {
    /// Enclave EventBus
    bus: BusHandle,
    /// NetCommand sender to forward commands to the Libp2pNetInterface
    tx: mpsc::Sender<NetCommand>,
    /// NetEvents receiver to receive events
    rx: Arc<broadcast::Receiver<NetEvent>>,
    eventstore: Recipient<EventStoreQueryBy<TsAgg>>,
    requests: HashMap<CorrelationId, DirectResponder>,
    all_peers_dialed: bool,
    has_connections: bool,
    net_ready_published: bool,
}

impl NetSyncManager {
    pub fn new(
        bus: &BusHandle,
        tx: &mpsc::Sender<NetCommand>,
        rx: &Arc<broadcast::Receiver<NetEvent>>,
        eventstore: Recipient<EventStoreQueryBy<TsAgg>>,
    ) -> Self {
        Self {
            bus: bus.clone(),
            tx: tx.clone(),
            rx: Arc::clone(rx),
            eventstore,
            requests: HashMap::new(),
            all_peers_dialed: false,
            has_connections: false,
            net_ready_published: false,
        }
    }

    fn publish_net_ready(&mut self) -> Result<()> {
        if !self.net_ready_published {
            self.net_ready_published = true;
            info!("NetSyncManager: publishing NetReady");
            self.bus.publish_without_context(NetReady::new())?;
        }
        Ok(())
    }

    pub fn setup(
        bus: &BusHandle,
        tx: &mpsc::Sender<NetCommand>,
        rx: &Arc<broadcast::Receiver<NetEvent>>,
        eventstore: Recipient<EventStoreQueryBy<TsAgg>>,
    ) -> Addr<Self> {
        let mut events = rx.resubscribe();
        let addr = Self::new(bus, tx, rx, eventstore).start();

        bus.subscribe(EventType::HistoricalNetSyncStart, addr.clone().recipient());

        // Forward from NetEvent
        tokio::spawn({
            debug!("Spawning event receive loop!");
            let addr = addr.clone();
            async move {
                while let Ok(event) = events.recv().await {
                    debug!("Received event {:?}", event);
                    match event {
                        // Someone is asking for our sync
                        NetEvent::IncomingRequest(value) => addr.do_send(value),
                        NetEvent::AllPeersDialed { connected, total } => {
                            addr.do_send(AllPeersDialed { connected, total })
                        }
                        NetEvent::ConnectionEstablished { .. } => addr.do_send(PeerConnected),
                        _ => (),
                    }
                }
            }
        });

        addr
    }
}

impl Actor for NetSyncManager {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT)
    }
}

/// Event broadcast from event bus
impl Handler<EnclaveEvent> for NetSyncManager {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        match msg {
            // We are making a sync request of another node
            EnclaveEventData::HistoricalNetSyncStart(data) => ctx.notify(TypedEvent::new(data, ec)),
            _ => (),
        }
    }
}

/// SyncRequest is called on start up to fetch remote events
impl Handler<TypedEvent<HistoricalNetSyncStart>> for NetSyncManager {
    type Result = ResponseFuture<()>;
    fn handle(
        &mut self,
        msg: TypedEvent<HistoricalNetSyncStart>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        info!("HISTORICAL_NET_SYNC_START");
        trap_fut(
            EType::Net,
            &self.bus.with_ec(msg.get_ctx()),
            handle_sync_request_event(
                self.tx.clone(),
                self.rx.clone(),
                msg,
                ctx.address(),
                !self.all_peers_dialed,
            ),
        )
    }
}

/// We have received the sync response from the remote peer
impl Handler<TypedEvent<SyncRequestSucceeded>> for NetSyncManager {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<SyncRequestSucceeded>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trap(EType::Net, &self.bus.with_ec(msg.get_ctx()), || {
            info!("SYNC REQUEST SUCCEEDED");
            let (msg, ctx) = msg.into_components();
            let response = msg.response;
            self.bus.publish_from_remote_as_response(
                HistoricalNetSyncEventsReceived {
                    events: response.events.iter().cloned().collect(),
                },
                response.ts,
                ctx,
                None,
                EventSource::Net,
            )?;

            Ok(())
        });
    }
}

/// We have received a sync request from a remote peer
impl Handler<IncomingRequest> for NetSyncManager {
    type Result = ();
    fn handle(&mut self, msg: IncomingRequest, ctx: &mut Self::Context) -> Self::Result {
        trap(EType::Net, &self.bus, || {
            let id = CorrelationId::new();
            info!("Processing incoming request with correlation={}", id);
            let fetch_request: FetchEventsSince = msg.responder.try_request_into()?;
            self.requests.insert(id, msg.responder);
            let query: HashMap<AggregateId, u128> =
                HashMap::from([(fetch_request.aggregate_id(), fetch_request.since())]);
            self.eventstore.try_send(EventStoreQueryBy::<TsAgg>::new(
                id,
                query,
                ctx.address().recipient(),
            ))?;
            Ok(())
        });
    }
}

/// Receive Events from EventStore
impl Handler<EventStoreQueryResponse> for NetSyncManager {
    type Result = ();
    fn handle(&mut self, msg: EventStoreQueryResponse, _: &mut Self::Context) -> Self::Result {
        trap(EType::Net, &self.bus.clone(), || {
            info!("Received response from eventstore.");
            let Some(responder) = self.requests.remove(&msg.id()) else {
                bail!("responder not found for {}", msg.id());
            };

            let fetch_request: FetchEventsSince = responder.try_request_into()?;
            let limit = fetch_request.limit();
            if limit == 0 {
                responder.bad_request("limit must be greater than 0")?;
                return Ok(());
            }
            let aggregate_id = fetch_request.aggregate_id();
            let events: Vec<EnclaveEvent<Unsequenced>> = msg
                .into_events()
                .into_iter()
                .filter(|e| e.source() == EventSource::Net)
                .take(limit)
                .map(|ev| ev.clone_unsequenced())
                .collect();

            let next = if events.len() == limit {
                let last_event_ts = events.last().map(|e| e.ts()).unwrap_or(0);
                BatchCursor::Next(last_event_ts)
            } else {
                BatchCursor::Done
            };

            responder.ok(EventBatch {
                events,
                next,
                aggregate_id,
            })?;

            Ok(())
        })
    }
}

impl Handler<AllPeersDialed> for NetSyncManager {
    type Result = ();
    fn handle(&mut self, msg: AllPeersDialed, ctx: &mut Self::Context) -> Self::Result {
        trap(EType::Sync, &self.bus.clone(), || {
            info!(
                "NetSyncManager: AllPeersDialed (connected={}, total={})",
                msg.connected, msg.total
            );
            self.all_peers_dialed = true;
            if msg.connected > 0 {
                self.has_connections = true;
            }
            if msg.total == 0 || self.has_connections {
                // No peers configured or connections already established
                self.publish_net_ready()?;
            } else {
                // All dials failed — wait for a ConnectionEstablished event.
                // Fall back to a 60-second timeout so we don't hang forever.
                info!(
                    "All peer dials failed, waiting for connections before publishing NetReady..."
                );
                let bus = self.bus.clone();
                ctx.run_later(NET_READY_CONNECT_TIMEOUT, move |this, _| {
                    if !this.net_ready_published {
                        warn!("No peer connections established within 60s timeout, publishing NetReady anyway");
                        this.net_ready_published = true;
                        if let Err(e) = bus.publish_without_context(NetReady::new()) {
                            error!("Failed to publish NetReady: {e}");
                        }
                    }
                });
            }
            Ok(())
        })
    }
}

impl Handler<PeerConnected> for NetSyncManager {
    type Result = ();
    fn handle(&mut self, _: PeerConnected, _: &mut Self::Context) -> Self::Result {
        if !self.has_connections {
            info!("NetSyncManager: first peer connected");
            self.has_connections = true;
            if self.all_peers_dialed {
                trap(EType::Sync, &self.bus.clone(), || self.publish_net_ready());
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct AllPeersDialed {
    connected: usize,
    total: usize,
}

#[derive(Message)]
#[rtype(result = "()")]
struct PeerConnected;

async fn handle_sync_request_event(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    event: TypedEvent<HistoricalNetSyncStart>,
    address: impl Into<Recipient<TypedEvent<SyncRequestSucceeded>>>,
    wait_for_event: bool,
) -> Result<()> {
    info!("Sync request event received");
    let (event, ctx) = event.into_components();
    info!("Checking for AllPeersDialed...");
    if wait_for_event {
        await_event(
            &net_events,
            |e| {
                if matches!(e, &NetEvent::AllPeersDialed { .. }) {
                    info!("AllPeersDialed matched!");
                    Some(e.clone())
                } else {
                    None
                }
            },
            ALL_PEERS_DIALED_TIMEOUT,
        )
        .await
        .ok(); // Timeout is non-fatal — proceed regardless
    }
    info!("handle_sync_request_event: ready to sync");

    let mut all_events: Vec<EnclaveEvent<Unsequenced>> = Vec::new();
    let mut latest_timestamp: u128 = 0;
    let mut failed_aggregates: Vec<AggregateId> = Vec::new();

    for (aggregate_id, since) in event.since.iter() {
        info!(
            "Requesting batched events for aggregate_id={} since={}",
            aggregate_id, since
        );
        let requester = DirectRequester::builder(net_cmds.clone(), net_events.clone()).build();
        match fetch_all_batched_events::<EnclaveEvent<Unsequenced>>(
            requester,
            PeerTarget::Random,
            *aggregate_id,
            *since,
            100,
        )
        .await
        {
            Ok(events) => {
                info!(
                    "Received {} events for aggregate_id={}",
                    events.len(),
                    aggregate_id
                );
                for enclave_event in events {
                    let ts = enclave_event.ts();
                    if ts > latest_timestamp {
                        latest_timestamp = ts;
                    }
                    all_events.push(enclave_event);
                }
            }
            Err(e) => {
                warn!(
                    "Failed to fetch events for aggregate_id={}: {e}. Continuing with available events.",
                    aggregate_id
                );
                failed_aggregates.push(*aggregate_id);
            }
        }
    }

    // If any aggregate failed (likely "no connected peers"), wait for a peer
    // to reconnect and retry. This handles the restart scenario where peers
    // briefly connect then immediately disconnect (the remote side still holds
    // the old connection and rejects the new one). Reconnection via Kademlia
    // can take up to ~120s.
    if !failed_aggregates.is_empty() {
        info!(
            "Sync fetch failed for {} aggregates — waiting for peer reconnection...",
            failed_aggregates.len()
        );
        match await_event(
            &net_events,
            |e| {
                if matches!(e, NetEvent::ConnectionEstablished { .. }) {
                    Some(())
                } else {
                    None
                }
            },
            SYNC_RECONNECT_TIMEOUT,
        )
        .await
        {
            Ok(()) => {
                info!("Peer reconnected, retrying failed aggregates");
                let mut still_failed = Vec::new();
                for aggregate_id in failed_aggregates {
                    let since = event.since.get(&aggregate_id).copied().unwrap_or(0);
                    let requester =
                        DirectRequester::builder(net_cmds.clone(), net_events.clone()).build();
                    match fetch_all_batched_events::<EnclaveEvent<Unsequenced>>(
                        requester,
                        PeerTarget::Random,
                        aggregate_id,
                        since,
                        100,
                    )
                    .await
                    {
                        Ok(events) => {
                            info!(
                                "Retry succeeded: {} events for aggregate_id={}",
                                events.len(),
                                aggregate_id
                            );
                            for enclave_event in events {
                                let ts = enclave_event.ts();
                                if ts > latest_timestamp {
                                    latest_timestamp = ts;
                                }
                                all_events.push(enclave_event);
                            }
                        }
                        Err(e) => {
                            warn!("Retry also failed for aggregate_id={}: {e}", aggregate_id);
                            still_failed.push(aggregate_id);
                        }
                    }
                }
                if !still_failed.is_empty() {
                    bail!(
                        "failed to fetch historical net events for aggregates: {:?}",
                        still_failed
                    );
                }
            }
            Err(_) => {
                bail!(
                    "failed to fetch historical net events for aggregates: {:?} (no peers reconnected within {:?})",
                    failed_aggregates,
                    SYNC_RECONNECT_TIMEOUT
                );
            }
        }
    }

    info!(
        "Sync complete: collected {} events across {} aggregates, latest_timestamp={}",
        all_events.len(),
        event.since.len(),
        latest_timestamp
    );

    let value = SyncRequestSucceeded {
        response: SyncResponseValue {
            events: all_events,
            ts: latest_timestamp,
        },
    };

    address.into().try_send(TypedEvent::new(value, ctx))?;
    Ok(())
}
