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
use e3_utils::{retry_with_backoff, to_retry, OnceTake, MAILBOX_LIMIT};
use futures::TryFutureExt;
use libp2p::request_response::ResponseChannel;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info, warn};

use crate::events::{
    await_event, call_and_await_response, GossipData, IncomingRequest, NetCommand, NetEvent,
    OutgoingRequestSucceeded, PeerTarget,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequestValue {
    pub since: HashMap<AggregateId, u128>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponseValue {
    pub events: Vec<GossipData>,
    pub ts: u128,
}

pub struct NetSyncManager {
    /// Enclave EventBus
    bus: BusHandle,
    /// NetCommand sender to forward commands to the NetInterface
    tx: mpsc::Sender<NetCommand>,
    /// NetEvents receiver to receive events
    rx: Arc<broadcast::Receiver<NetEvent>>,
    eventstore: Recipient<EventStoreQueryBy<TsAgg>>,
    requests: HashMap<CorrelationId, OnceTake<ResponseChannel<Vec<u8>>>>,
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
impl Handler<TypedEvent<OutgoingRequestSucceeded>> for NetSyncManager {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<OutgoingRequestSucceeded>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trap(EType::Net, &self.bus.with_ec(msg.get_ctx()), || {
            let (msg, ctx) = msg.into_components();
            let response: SyncResponseValue = bincode::deserialize(&msg.payload)
                .context("failed to deserialize sync response")?;
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
            info!("GOT SyncRequestReceived");
            let request: SyncRequestValue =
                bincode::deserialize(&msg.payload).context("failed to deserialize sync request")?;
            let id = CorrelationId::new();
            info!("STORING channel in requests map...");
            self.requests.insert(id, msg.channel);
            info!("QUERYING eventstore...");
            self.eventstore.try_send(EventStoreQueryBy::<TsAgg>::new(
                id,
                request.since,
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
            let Some(channel) = self.requests.get(&msg.id()) else {
                bail!("request not found with {}", msg.id());
            };
            debug!("Sending SyncResponse with channel={:?}", channel);
            let response = SyncResponseValue {
                events: msg
                    .into_events()
                    .into_iter()
                    .filter(|e| e.source() == EventSource::Net)
                    .map(|ev| ev.try_into())
                    .collect::<Result<_>>()?,
                ts: self.bus.ts()?, // NOTE: We are storing a local timestamp on this response
            };
            let payload =
                bincode::serialize(&response).context("failed to serialize sync response")?;
            if let Err(e) = self.tx.try_send(NetCommand::Response {
                payload,
                channel: channel.to_owned(),
            }) {
                warn!("Failed to send SyncResponse (channel full or closed): {e}");
            }

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
) -> Result<OutgoingRequestSucceeded> {
    info!("RUNNING sync request...");
    let id = CorrelationId::new();
    let payload = bincode::serialize(&SyncRequestValue { since })
        .context("failed to serialize sync request")?;
    call_and_await_response(
        net_cmds,
        net_events,
        NetCommand::OutgoingRequest {
            correlation_id: id,
            payload,
            target: PeerTarget::Random,
        },
        |e| match e.clone() {
            NetEvent::OutgoingRequestSucceeded(value) => Some(Ok(value)),
            NetEvent::OutgoingRequestFailed(error) => {
                Some(Err(anyhow!("Outgoing sync request failed: {:?}", error)))
            }
            _ => None,
        },
        SYNC_REQUEST_TIMEOUT,
    )
    .await
}

async fn handle_sync_request_event(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    event: TypedEvent<HistoricalNetSyncStart>,
    address: impl Into<Recipient<TypedEvent<OutgoingRequestSucceeded>>>,
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
