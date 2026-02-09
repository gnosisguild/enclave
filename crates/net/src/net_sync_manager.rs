// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Addr, AsyncContext, Handler, Recipient, ResponseFuture};
use anyhow::{anyhow, bail, Result};
use e3_events::{
    prelude::*, trap, trap_fut, AggregateId, BusHandle, CorrelationId, EType, EnclaveEvent,
    EnclaveEventData, EventStoreQueryBy, EventStoreQueryResponse, EventType,
    HistoricalNetSyncStart, NetSyncEventsReceived, TsAgg, TypedEvent, Unsequenced,
};
use e3_utils::{retry_with_backoff, to_retry, OnceTake, MAILBOX_LIMIT};
use futures::TryFutureExt;
use libp2p::request_response::ResponseChannel;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{broadcast, mpsc};
use tracing::debug;

use crate::events::{
    call_and_await_response, NetCommand, NetEvent, OutgoingSyncRequestSucceeded,
    SyncRequestReceived, SyncRequestValue, SyncResponseValue,
};

pub struct NetSyncManager {
    /// Enclave EventBus
    bus: BusHandle,
    /// NetCommand sender to forward commands to the NetInterface
    tx: mpsc::Sender<NetCommand>,
    /// NetEvents receiver to receive events
    rx: Arc<broadcast::Receiver<NetEvent>>,
    eventstore: Recipient<EventStoreQueryBy<TsAgg>>,
    requests: HashMap<CorrelationId, OnceTake<ResponseChannel<SyncResponseValue>>>,
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
                        NetEvent::SyncRequestReceived(value) => addr.do_send(value),
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
        trap_fut(
            EType::Net,
            &self.bus.with_ec(msg.get_ctx()),
            handle_sync_request_event(self.tx.clone(), self.rx.clone(), msg, ctx.address()),
        )
    }
}

/// We have received the sync response from the remote peer
impl Handler<TypedEvent<OutgoingSyncRequestSucceeded>> for NetSyncManager {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<OutgoingSyncRequestSucceeded>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trap(EType::Net, &self.bus.with_ec(msg.get_ctx()), || {
            let (msg, ctx) = msg.into_components();
            self.bus.publish_from_remote_as_response(
                NetSyncEventsReceived {
                    events: msg
                        .value
                        .events
                        .iter()
                        .cloned()
                        .map(|data| data.try_into())
                        .collect::<Result<Vec<EnclaveEvent<Unsequenced>>>>()?,
                },
                msg.value.ts,
                ctx,
                None,
            )?;

            Ok(())
        });
    }
}

/// We have received a sync request from a remote peer
impl Handler<SyncRequestReceived> for NetSyncManager {
    type Result = ();
    fn handle(&mut self, msg: SyncRequestReceived, ctx: &mut Self::Context) -> Self::Result {
        trap(EType::Net, &self.bus, || {
            let id = CorrelationId::new();
            self.requests.insert(id, msg.channel);
            self.eventstore.try_send(EventStoreQueryBy::<TsAgg>::new(
                id,
                msg.value.since,
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
            let Some(channel) = self.requests.get(&msg.id()) else {
                bail!("request not found with {}", msg.id());
            };

            self.tx.try_send(NetCommand::SyncResponse {
                value: SyncResponseValue {
                    events: msg
                        .into_events()
                        .into_iter()
                        .map(|ev| ev.try_into())
                        .collect::<Result<_>>()?,
                    ts: self.bus.ts()?, // NOTE: We are storing a local timestamp on this response
                },
                channel: channel.to_owned(),
            })?;

            Ok(())
        })
    }
}

const SYNC_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

async fn sync_request(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    since: HashMap<AggregateId, u128>,
) -> Result<OutgoingSyncRequestSucceeded> {
    call_and_await_response(
        net_cmds,
        net_events,
        NetCommand::OutgoingSyncRequest {
            correlation_id: CorrelationId::new(),
            value: SyncRequestValue { since },
        },
        |e| match e.clone() {
            NetEvent::OutgoingSyncRequestSucceeded(value) => Some(Ok(value)),
            NetEvent::OutgoingSyncRequestFailed(error) => {
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
    address: impl Into<Recipient<TypedEvent<OutgoingSyncRequestSucceeded>>>,
) -> Result<()> {
    let (event, ctx) = event.into_components();

    // Make the sync request
    // value returned includes the timestamp from the remote peer
    let value = retry_with_backoff(
        || {
            sync_request(
                net_cmds.clone(),
                net_events.clone(),
                event.since.clone().into_iter().collect(),
            )
            .map_err(to_retry)
        },
        4,
        1000,
    )
    .await?;

    // send the sync request succeeded to ourselves
    address.into().try_send(TypedEvent::new(value, ctx))?;
    Ok(())
}
