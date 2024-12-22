use crate::correlation_id::CorrelationId;
use crate::events::NetworkPeerCommand;
use crate::events::NetworkPeerEvent;
use crate::NetworkPeer;
/// Actor for connecting to an libp2p client via it's mpsc channel interface
/// This Actor should be responsible for
use actix::prelude::*;
use anyhow::{bail, Result};
use cipher::Cipher;
use data::Repository;
use enclave_core::{EnclaveEvent, EventBus, EventId, Subscribe};
use libp2p::identity::ed25519;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::{error, info, instrument, trace};

/// NetworkManager Actor converts between EventBus events and Libp2p events forwarding them to a
/// NetworkPeer for propagation over the p2p network
pub struct NetworkManager {
    bus: Addr<EventBus>,
    tx: mpsc::Sender<NetworkPeerCommand>,
    sent_events: HashSet<EventId>,
    topic: String,
}

impl Actor for NetworkManager {
    type Context = Context<Self>;
}

/// Libp2pEvent is used to send data to the NetworkPeer from the NetworkManager
#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "anyhow::Result<()>")]
struct LibP2pEvent(pub Vec<u8>);

impl NetworkManager {
    /// Create a new NetworkManager actor
    pub fn new(bus: Addr<EventBus>, tx: mpsc::Sender<NetworkPeerCommand>, topic: &str) -> Self {
        Self {
            bus,
            tx,
            sent_events: HashSet::new(),
            topic: topic.to_string(),
        }
    }

    pub fn setup(
        bus: Addr<EventBus>,
        tx: mpsc::Sender<NetworkPeerCommand>,
        mut rx: broadcast::Receiver<NetworkPeerEvent>,
        topic: &str,
    ) -> Addr<Self> {
        let addr = NetworkManager::new(bus.clone(), tx, topic).start();

        // Listen on all events
        bus.do_send(Subscribe {
            event_type: String::from("*"),
            listener: addr.clone().recipient(),
        });

        tokio::spawn({
            let addr = addr.clone();
            async move {
                loop {
                    select! {
                        Ok(event) = rx.recv() => {
                            match event {
                                NetworkPeerEvent::GossipData(data) => {
                                    addr.do_send(LibP2pEvent(data))
                                },
                                _ => ()
                            }
                        }
                    }
                }
            }
        });

        addr
    }

    /// Spawn a Libp2p peer and hook it up to this actor
    #[instrument(name = "libp2p", skip_all)]
    pub async fn setup_with_peer(
        bus: Addr<EventBus>,
        peers: Vec<String>,
        cipher: &Arc<Cipher>,
        quic_port: u16,
        enable_mdns: bool,
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
        let mut peer = NetworkPeer::new(&keypair, peers, Some(quic_port), topic, enable_mdns)?;

        // Setup and start network manager
        let rx = peer.rx();
        let p2p_addr = NetworkManager::setup(bus, peer.tx(), rx, topic);
        let handle = tokio::spawn(async move { Ok(peer.start().await?) });

        Ok((p2p_addr, handle, keypair.public().to_peer_id().to_string()))
    }
}

impl Handler<LibP2pEvent> for NetworkManager {
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

impl Handler<EnclaveEvent> for NetworkManager {
    type Result = ResponseFuture<()>;
    fn handle(&mut self, event: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        let sent_events = self.sent_events.clone();
        let tx = self.tx.clone();
        let evt = event.clone();
        let topic = self.topic.clone();
        Box::pin(async move {
            let id: EventId = evt.clone().into();

            // if we have seen this event before dont rebroadcast
            if sent_events.contains(&id) {
                trace!(evt_id=%id,"Have seen event before not rebroadcasting!");
                return;
            }

            // Ignore events that should be considered local
            if evt.is_local_only() {
                trace!(evt_id=%id,"Local events should not be rebroadcast so ignoring");
                return;
            }

            match evt.to_bytes() {
                Ok(data) => {
                    if let Err(e) = tx
                        .send(NetworkPeerCommand::GossipPublish {
                            topic,
                            data,
                            correlation_id: CorrelationId::new(),
                        })
                        .await
                    {
                        error!(error=?e, "Error sending bytes to libp2p");
                    };
                }
                Err(error) => {
                    error!(error=?error, "Could not convert event to bytes for serialization!")
                }
            }
        })
    }
}
