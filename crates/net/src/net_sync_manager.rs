// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Addr, AsyncContext, Handler, Message, Recipient, ResponseFuture};
use anyhow::{anyhow, bail, Context, Result};
use e3_events::{
    prelude::*, trap, trap_fut, AggregateId, BusHandle, CorrelationId, EType, EnclaveEvent,
    EnclaveEventData, EventSource, EventStoreQueryBy, EventStoreQueryResponse, EventType,
    HistoricalNetSyncStart, NetSyncEventsReceived, TsAgg, TypedEvent, Unsequenced,
};
use e3_utils::{retry_with_backoff, to_retry, MAILBOX_LIMIT};
use futures::TryFutureExt;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::TryInto, sync::Arc, time::Duration};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info};

use crate::{
    direct_responder::DirectResponder,
    events::{
        await_event, call_and_await_response, GossipData, IncomingRequest, NetCommand, NetEvent,
        OutgoingRequest, ProtocolResponse,
    },
    net_event_batch::{BatchCursor, EventBatch, FetchEventsSince},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequestValue {
    pub since: HashMap<AggregateId, u128>,
}

impl TryInto<Vec<u8>> for SyncRequestValue {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        bincode::serialize(&self).context("failed to serialize SyncRequestValue")
    }
}

impl TryFrom<Vec<u8>> for SyncRequestValue {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        bincode::deserialize(&value).context("failed to deserialize SyncRequestValue")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponseValue {
    pub events: Vec<GossipData>,
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
    /// NetCommand sender to forward commands to the NetInterface
    tx: mpsc::Sender<NetCommand>,
    /// NetEvents receiver to receive events
    rx: Arc<broadcast::Receiver<NetEvent>>,
    eventstore: Recipient<EventStoreQueryBy<TsAgg>>,
    requests: HashMap<CorrelationId, DirectResponder>,
    peers_ready: bool,
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
            peers_ready: false,
        }
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
                        NetEvent::AllPeersDialed => addr.do_send(AllPeersDialed),
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
                !self.peers_ready,
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
            let (msg, ctx) = msg.into_components();
            let response = msg.response;
            self.bus.publish_from_remote_as_response(
                NetSyncEventsReceived {
                    events: response
                        .events
                        .iter()
                        .cloned()
                        .map(|data| data.try_into())
                        .collect::<Result<Vec<EnclaveEvent<Unsequenced>>>>()?,
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
            let aggregate_id = fetch_request.aggregate_id();
            let events: Vec<EnclaveEvent<Unsequenced>> = msg
                .into_events()
                .into_iter()
                .filter(|e| e.source() == EventSource::Net)
                .take(limit)
                .map(|ev| ev.clone_unsequenced())
                .collect();

            let next = if events.len() == limit {
                let last_event_ts = events.get(limit - 1).map(|e| e.ts()).unwrap_or(0);
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
    fn handle(&mut self, _: AllPeersDialed, _: &mut Self::Context) -> Self::Result {
        info!("Received handler: All peers dialed");
        self.peers_ready = true;
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct AllPeersDialed;

const SYNC_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

async fn sync_request(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    since: HashMap<AggregateId, u128>,
) -> Result<SyncRequestSucceeded> {
    info!("RUNNING sync request...");
    let response = call_and_await_response(
        net_cmds,
        net_events,
        NetCommand::OutgoingRequest(OutgoingRequest::to_random_peer(SyncRequestValue { since })?),
        |e| match e.clone() {
            NetEvent::OutgoingRequestSucceeded(value) => Some(Ok(value)),
            NetEvent::OutgoingRequestFailed(error) => {
                Some(Err(anyhow!("Outgoing sync request failed: {:?}", error)))
            }
            _ => None,
        },
        SYNC_REQUEST_TIMEOUT,
    )
    .await?;
    match response.payload {
        ProtocolResponse::Ok(data) => {
            let response: SyncResponseValue = data.try_into()?;
            Ok(SyncRequestSucceeded { response })
        }
        ProtocolResponse::BadRequest(msg) => Err(anyhow!("BadRequest: {}", msg)),
        ProtocolResponse::Error(msg) => Err(anyhow!("ProtocolError: {}", msg)),
    }
}

async fn handle_sync_request_event(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    event: TypedEvent<HistoricalNetSyncStart>,
    address: impl Into<Recipient<TypedEvent<SyncRequestSucceeded>>>,
    wait_for_event: bool,
) -> Result<()> {
    info!("Sync request event received");
    let (event, ctx) = event.into_components();
    info!("Waiting for peers to have been contacted...");
    if wait_for_event {
        await_event(
            &net_events,
            |e| {
                if matches!(e, &NetEvent::AllPeersDialed) {
                    info!("AllPeersDialed matched!");
                    Some(e.clone())
                } else {
                    None
                }
            },
            Duration::from_secs(30),
        )
        .await?;
    }
    info!("handle_sync_request_event: All peers have been dialed.");

    // Make the sync request
    // value returned includes the timestamp from the remote peer
    let value = retry_with_backoff(
        || {
            info!("Running SYNC REQUEST!!");
            sync_request(
                net_cmds.clone(),
                net_events.clone(),
                event.since.clone().into_iter().collect(),
            )
            .map_err(to_retry)
        },
        4,
        5000,
    )
    .await?;

    // send the sync request succeeded to ourselves
    address.into().try_send(TypedEvent::new(value, ctx))?;
    Ok(())
}
