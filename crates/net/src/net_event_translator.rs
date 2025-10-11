// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::events::GossipData;
use crate::events::NetCommand;
use crate::events::NetEvent;
use crate::NetInterface;
/// Actor for connecting to an libp2p client via it's mpsc channel interface
/// This Actor should be responsible for
use actix::prelude::*;
use anyhow::{bail, Result};
use e3_crypto::Cipher;
use e3_data::Repository;
use e3_events::{CorrelationId, EnclaveEvent, EventBus, EventId, Subscribe};
use libp2p::identity::ed25519;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::{error, info, instrument, trace};

// TODO: store event filtering here on this actor instead of is_local_only() on the event. We
// should do this as this functionality is not global and ramifications should stay local to here

/// NetEventTranslator Actor converts between EventBus events and Libp2p events forwarding them to a
/// NetInterface for propagation over the p2p network
pub struct NetEventTranslator {
    bus: Addr<EventBus<EnclaveEvent>>,
    tx: mpsc::Sender<NetCommand>,
    sent_events: HashSet<EventId>,
    topic: String,
}

impl Actor for NetEventTranslator {
    type Context = Context<Self>;
}

/// Libp2pEvent is used to send data to the NetInterface from the NetEventTranslator
#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "anyhow::Result<()>")]
struct LibP2pEvent(pub Vec<u8>);

impl NetEventTranslator {
    /// Create a new NetEventTranslator actor
    pub fn new(
        bus: Addr<EventBus<EnclaveEvent>>,
        tx: &mpsc::Sender<NetCommand>,
        topic: &str,
    ) -> Self {
        Self {
            bus,
            tx: tx.clone(),
            sent_events: HashSet::new(),
            topic: topic.to_string(),
        }
    }

    pub fn setup(
        bus: Addr<EventBus<EnclaveEvent>>,
        tx: &mpsc::Sender<NetCommand>,
        mut rx: broadcast::Receiver<NetEvent>,
        topic: &str,
    ) -> Addr<Self> {
        let addr = NetEventTranslator::new(bus.clone(), tx, topic).start();

        // Listen on all events
        bus.do_send(Subscribe {
            event_type: String::from("*"),
            listener: addr.clone().recipient(),
        });

        tokio::spawn({
            let addr = addr.clone();
            async move {
                while let Ok(event) = rx.recv().await {
                    match event {
                        NetEvent::GossipData(data) => {
                            if let GossipData::GossipBytes(payload) = data {
                                addr.do_send(LibP2pEvent(payload));
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
        match event {
            EnclaveEvent::KeyshareCreated { .. } => true,
            EnclaveEvent::PublicKeyAggregated { .. } => true,
            EnclaveEvent::DecryptionshareCreated { .. } => true,
            EnclaveEvent::PlaintextAggregated { .. } => true,
            _ => false,
        }
    }

    /// Spawn a Libp2p interface and hook it up to this actor
    #[instrument(name = "libp2p", skip_all)]
    pub async fn setup_with_interface(
        bus: Addr<EventBus<EnclaveEvent>>,
        peers: Vec<String>,
        cipher: &Arc<Cipher>,
        quic_port: u16,
        repository: Repository<Vec<u8>>,
    ) -> Result<(Addr<Self>, tokio::task::JoinHandle<Result<()>>, String)> {
        let topic = "tmp-enclave-gossip-topic";
        // Get existing keypair or generate a new one
        let mut bytes = match repository.read().await? {
            Some(bytes) => {
                info!("Found keypair in repository");
                cipher.decrypt_data(&bytes)?
            }
            None => bail!("No network keypair found in repository, please generate a new one using `enclave net generate-key`"),
        };

        // Create peer from keypair
        let keypair: libp2p::identity::Keypair =
            ed25519::Keypair::try_from_bytes(&mut bytes)?.try_into()?;
        let mut interface = NetInterface::new(&keypair, peers, Some(quic_port), topic)?;

        // Setup and start net event translator
        let rx = interface.rx();
        let addr = NetEventTranslator::setup(bus, &interface.tx(), rx, topic);
        let handle = tokio::spawn(async move { Ok(interface.start().await?) });

        Ok((addr, handle, keypair.public().to_peer_id().to_string()))
    }
}

impl Handler<LibP2pEvent> for NetEventTranslator {
    type Result = anyhow::Result<()>;
    fn handle(&mut self, msg: LibP2pEvent, _: &mut Self::Context) -> Self::Result {
        let LibP2pEvent(bytes) = msg;
        match EnclaveEvent::from_bytes(&bytes) {
            Ok(event) => {
                self.bus.do_send(event.clone());
                self.sent_events.insert(event.into());
            }
            Err(err) => error!(error=?err, "Could not create EnclaveEvent from Libp2p Bytes!"),
        }
        Ok(())
    }
}

impl Handler<EnclaveEvent> for NetEventTranslator {
    type Result = ResponseFuture<()>;
    fn handle(&mut self, event: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        let sent_events = self.sent_events.clone();
        let tx = self.tx.clone();
        let evt = event.clone();
        let topic = self.topic.clone();
        Box::pin(async move {
            let id: EventId = evt.clone().into();

            // Ignore events that should be considered local
            if !Self::is_forwardable_event(&evt) {
                trace!(evt_id=%id,"Local events should not be rebroadcast so ignoring");
                return;
            }

            // if we have seen this event before dont rebroadcast
            if sent_events.contains(&id) {
                trace!(evt_id=%id,"Have seen event before not rebroadcasting!");
                return;
            }

            match evt.to_bytes() {
                Ok(data) => {
                    if let Err(e) = tx
                        .send(NetCommand::GossipPublish {
                            topic,
                            data: GossipData::GossipBytes(data),
                            correlation_id: CorrelationId::new(),
                        })
                        .await
                    {
                        error!(error=?e, "Error sending bytes to libp2p: {e}");
                    };
                }
                Err(error) => {
                    error!(error=?error, "Could not convert event to bytes for serialization!")
                }
            }
        })
    }
}
