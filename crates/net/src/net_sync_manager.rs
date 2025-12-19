use actix::{Actor, Addr, AsyncContext, Handler, Recipient};
use anyhow::{bail, Result};
use e3_events::{
    prelude::*, trap, BusHandle, CorrelationId, EType, EnclaveEvent, EnclaveEventData, Event,
    GetEventsAfter, NetEventsReceived, ReceiveEvents, SyncRequest, Unsequenced,
};
use e3_utils::OnceTake;
use libp2p::request_response::ResponseChannel;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, mpsc};
use tracing::debug;

use crate::events::{
    NetCommand, NetEvent, OutgoingSyncRequestSucceeded, SyncRequestReceived, SyncRequestValue,
    SyncResponseValue,
};

pub struct NetSyncManager {
    /// Enclave EventBus
    bus: BusHandle,
    /// NetCommand sender to forward commands to the NetInterface
    tx: mpsc::Sender<NetCommand>,
    eventstore: Recipient<GetEventsAfter>,
    requests: HashMap<CorrelationId, OnceTake<ResponseChannel<SyncResponseValue>>>,
}

impl NetSyncManager {
    pub fn new(
        bus: &BusHandle,
        tx: &mpsc::Sender<NetCommand>,
        eventstore: Recipient<GetEventsAfter>,
    ) -> Self {
        Self {
            bus: bus.clone(),
            tx: tx.clone(),
            eventstore,
            requests: HashMap::new(),
        }
    }

    pub fn setup(
        bus: &BusHandle,
        tx: &mpsc::Sender<NetCommand>,
        rx: &Arc<broadcast::Receiver<NetEvent>>,
        eventstore: Recipient<GetEventsAfter>,
    ) -> Addr<Self> {
        let mut events = rx.resubscribe();
        let addr = Self::new(bus, tx, eventstore).start();

        // Forward from NetEvent
        tokio::spawn({
            debug!("Spawning event receive loop!");
            let addr = addr.clone();
            async move {
                while let Ok(event) = events.recv().await {
                    debug!("Received event {:?}", event);
                    match event {
                        NetEvent::OutgoingSyncRequestSucceeded(value) => addr.do_send(value),
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
}

/// Event broadcast from event bus
impl Handler<EnclaveEvent> for NetSyncManager {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg.into_data() {
            EnclaveEventData::SyncRequest(data) => ctx.notify(data),
            _ => (),
        }
    }
}

/// SyncRequest is called on start up to fetch remote events
impl Handler<SyncRequest> for NetSyncManager {
    type Result = ();
    fn handle(&mut self, msg: SyncRequest, _: &mut Self::Context) -> Self::Result {
        trap(EType::Net, &self.bus, || {
            self.tx.try_send(NetCommand::OutgoingSyncRequest {
                value: SyncRequestValue { since: msg.since },
            })?;
            Ok(())
        });
    }
}

/// We have received the sync response from the remote peer
impl Handler<OutgoingSyncRequestSucceeded> for NetSyncManager {
    type Result = ();
    fn handle(&mut self, msg: OutgoingSyncRequestSucceeded, _: &mut Self::Context) -> Self::Result {
        trap(EType::Net, &self.bus.clone(), || {
            self.bus.publish(NetEventsReceived {
                events: msg
                    .value
                    .events
                    .iter()
                    .cloned()
                    .map(|data| data.try_into())
                    .collect::<Result<Vec<EnclaveEvent<Unsequenced>>>>()?,
            })?;

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
            self.eventstore
                .try_send(GetEventsAfter::new(id, ctx.address(), msg.value.since))?;
            Ok(())
        });
    }
}

/// Receive Events from EventStore
impl Handler<ReceiveEvents> for NetSyncManager {
    type Result = ();
    fn handle(&mut self, msg: ReceiveEvents, _: &mut Self::Context) -> Self::Result {
        trap(EType::Net, &self.bus.clone(), || {
            let Some(channel) = self.requests.get(&msg.id()) else {
                bail!("request not found with {}", msg.id());
            };

            self.tx.try_send(NetCommand::SyncResponse {
                value: SyncResponseValue {
                    events: msg
                        .events()
                        .into_iter()
                        .cloned()
                        .map(|ev| ev.try_into())
                        .collect::<Result<_>>()?,
                },
                channel: channel.to_owned(),
            })?;

            Ok(())
        })
    }
}
