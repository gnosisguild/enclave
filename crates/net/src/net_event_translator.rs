// SPDX-License-Identifier: LGPL-3.0-only
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::events::GossipData;
use crate::events::NetCommand;
use crate::events::NetEvent;
use crate::net_event_buffer::NetEventBuffer;
use crate::net_sync_manager::NetSyncManager;
use crate::DocumentPublisher;
use crate::NetInterface;
/// Actor for connecting to an libp2p client via it's mpsc channel interface
/// This Actor should be responsible for
use actix::prelude::*;
use anyhow::{bail, Result};
use e3_crypto::Cipher;
use e3_data::Repository;
use e3_events::prelude::*;
use e3_events::trap;
use e3_events::BusHandle;
use e3_events::EType;
use e3_events::EnclaveEventData;
use e3_events::Event;
use e3_events::EventContextAccessors;
use e3_events::EventStoreQueryBy;
use e3_events::EventType;
use e3_events::TsAgg;
use e3_events::Unsequenced;
use e3_events::{CorrelationId, EnclaveEvent, EventId};
use e3_utils::MAILBOX_LIMIT;
use libp2p::identity::ed25519;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::warn;
use tracing::{info, instrument, trace};

// TODO: store event filtering here on this actor instead of is_local_only() on the event. We
// should do this as this functionality is not global and ramifications should stay local to here

/// NetEventTranslator Actor converts between EventBus events and Libp2p events forwarding them to a
/// NetInterface for propagation over the p2p network
pub struct NetEventTranslator {
    bus: BusHandle,
    tx: mpsc::Sender<NetCommand>,
    sent_events: HashSet<EventId>,
    topic: String,
}

impl Actor for NetEventTranslator {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
    }
}

/// Libp2pEvent is used to send data to the NetInterface from the NetEventTranslator
#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
struct LibP2pEvent(pub GossipData);

impl NetEventTranslator {
    /// Create a new NetEventTranslator actor
    pub fn new(bus: &BusHandle, tx: &mpsc::Sender<NetCommand>, topic: &str) -> Self {
        Self {
            bus: bus.clone(),
            tx: tx.clone(),
            sent_events: HashSet::new(),
            topic: topic.to_string(),
        }
    }

    pub fn setup(
        bus: &BusHandle,
        tx: &mpsc::Sender<NetCommand>,
        rx: &Arc<broadcast::Receiver<NetEvent>>,
        topic: &str,
    ) -> Addr<Self> {
        let mut rx = rx.resubscribe();
        let addr = NetEventTranslator::new(&bus, tx, topic).start();

        // Listen on all events
        bus.subscribe(EventType::All, addr.clone().recipient());

        tokio::spawn({
            let addr = addr.clone();
            async move {
                while let Ok(event) = rx.recv().await {
                    match event {
                        NetEvent::GossipData(data) => {
                            if let GossipData::GossipBytes(_) = data {
                                addr.do_send(LibP2pEvent(data));
                            }
                        }
                        _ => (),
                    }
                }
            }
        });

        addr
    }

    /// Function to determine which events are allowed to be automatically
    /// broadcast to the network by the NetEventTranslator. Having this function
    /// as static means we can keep this maintained here but use this rule elsewhere
    pub fn is_forwardable_event(event: &EnclaveEvent) -> bool {
        // Add a list of events allowed to be forwarded to libp2p
        match event.get_data() {
            EnclaveEventData::DecryptionshareCreated(_) => true,
            EnclaveEventData::KeyshareCreated(_) => true,
            EnclaveEventData::PlaintextAggregated(_) => true,
            EnclaveEventData::PublicKeyAggregated(_) => true,
            _ => false,
        }
    }

    fn process_gossip_event(&mut self, msg: EnclaveEvent) -> Result<()> {
        // if we have seen this event before dont rebroadcast
        let id = msg.event_id();
        if self.sent_events.contains(&id) {
            trace!(evt_id=%id,"Have seen event before not rebroadcasting!");
            return Ok(());
        }

        warn!("GossipPublish event: {}", msg.event_type());
        let topic = self.topic.clone();
        let data: GossipData = msg.try_into()?;

        self.tx.try_send(NetCommand::GossipPublish {
            topic,
            data,
            correlation_id: CorrelationId::new(),
        })?;

        Ok(())
    }

    fn handle_enclave_event(&mut self, msg: EnclaveEvent) -> Result<()> {
        // Ignore events that should be considered local
        if !Self::is_forwardable_event(&msg) {
            let id = msg.event_id();
            trace!(evt_id=%id,"Local events should not be rebroadcast so ignoring");
            return Ok(());
        }

        self.process_gossip_event(msg)?;

        Ok(())
    }

    fn publish_event(&mut self, event: EnclaveEvent<Unsequenced>) -> Result<()> {
        let id = event.id();
        let (data, ec) = event.into_components();
        self.bus.publish_from_remote(data, ec.ts(), None)?;
        self.sent_events.insert(id);
        Ok(())
    }

    fn handle_remote_event(&mut self, msg: LibP2pEvent) -> Result<()> {
        let data = msg.0;
        let event: EnclaveEvent<Unsequenced> = data.try_into()?;
        self.publish_event(event)?;
        Ok(())
    }
}

impl Handler<LibP2pEvent> for NetEventTranslator {
    type Result = ();
    fn handle(&mut self, msg: LibP2pEvent, _: &mut Self::Context) -> Self::Result {
        trap(EType::Net, &self.bus.clone(), || {
            self.handle_remote_event(msg)
        })
    }
}

impl Handler<EnclaveEvent> for NetEventTranslator {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        trap(EType::Net, &self.bus.with_ec(msg.get_ctx()), || {
            self.handle_enclave_event(msg)
        })
    }
}
