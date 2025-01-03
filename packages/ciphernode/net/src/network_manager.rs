use crate::correlation_id::CorrelationId;
use crate::dialer::DialerActor;
use crate::events::{NetworkPeerCommand, NetworkPeerEvent};
use crate::network_peer::NetworkPeer;

/// Actor for connecting to an libp2p client via it's mpsc channel interface
/// This Actor should be responsible for
use actix::prelude::*;
use anyhow::{bail, Result};
use crypto::Cipher;
use data::Repository;
use events::EventBusConfig;
use events::{EnclaveEvent, EventBus, EventId, Subscribe};
use libp2p::{gossipsub, identity::ed25519};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, instrument, trace, warn};

/// NetworkManager Actor converts between EventBus events and Libp2p events forwarding them to a
/// NetworkPeer for propagation over the p2p network
pub struct NetworkManager {
    bus: Addr<EventBus<EnclaveEvent>>,
    net_bus: Addr<EventBus<NetworkPeerEvent>>,
    tx: mpsc::Sender<NetworkPeerCommand>,
    sent_events: HashSet<EventId>,
    topic: String,
}

impl Actor for NetworkManager {
    type Context = Context<Self>;
}

impl NetworkManager {
    /// Create a new NetworkManager actor
    pub fn new(
        bus: Addr<EventBus<EnclaveEvent>>,
        net_bus: Addr<EventBus<NetworkPeerEvent>>,
        tx: mpsc::Sender<NetworkPeerCommand>,
        topic: &str,
    ) -> Self {
        Self {
            bus,
            net_bus,
            tx,
            sent_events: HashSet::new(),
            topic: topic.to_string(),
        }
    }

    pub fn setup(
        bus: Addr<EventBus<EnclaveEvent>>,
        net_bus: Addr<EventBus<NetworkPeerEvent>>,
        tx: mpsc::Sender<NetworkPeerCommand>,
        topic: &str,
    ) -> Addr<Self> {
        let addr = NetworkManager::new(bus.clone(), net_bus.clone(), tx, topic).start();

        // Listen on all events
        bus.do_send(Subscribe {
            event_type: String::from("*"),
            listener: addr.clone().recipient(),
        });

        net_bus.do_send(Subscribe {
            event_type: String::from("*"),
            listener: addr.clone().recipient(),
        });

        addr
    }

    /// Spawn a Libp2p peer and hook it up to this actor
    #[instrument(name = "libp2p", skip_all)]
    pub async fn setup_with_peer(
        bus: Addr<EventBus<EnclaveEvent>>,
        peers: Vec<String>,
        cipher: &Arc<Cipher>,
        quic_port: u16,
        enable_mdns: bool,
        repository: Repository<Vec<u8>>,
    ) -> Result<(Addr<Self>, tokio::task::JoinHandle<Result<()>>, String)> {
        let net_bus = EventBus::<NetworkPeerEvent>::new(EventBusConfig {
            capture_history: true,
            deduplicate: false,
        })
        .start();
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

        // Create Channel for Dialer
        let (tx, rx) = mpsc::channel(100);
        let mut swarm_manager = match NetworkPeer::new(&keypair, enable_mdns, net_bus.clone(), rx)
        {
            Ok(swarm_manager) => swarm_manager,
            Err(e) => {
                warn!("Failed to create NetworkPeer: {:?}", e);
                return Err(e);
            }
        };
        let topic = gossipsub::IdentTopic::new(topic);
        swarm_manager.subscribe(&topic)?;
        swarm_manager.listen_on(quic_port)?;

        let handle = tokio::spawn(async move { Ok(swarm_manager.start().await?) });
        for peer in peers {
            DialerActor::dial_peer(peer, net_bus.clone(), tx.clone());
        }

        let p2p_addr = NetworkManager::setup(bus, net_bus, tx, &topic.to_string());

        Ok((p2p_addr, handle, keypair.public().to_peer_id().to_string()))
    }
}

impl Handler<NetworkPeerEvent> for NetworkManager {
    type Result = ();
    fn handle(&mut self, msg: NetworkPeerEvent, _: &mut Self::Context) -> Self::Result {
        match msg {
            NetworkPeerEvent::GossipData(data) => match EnclaveEvent::from_bytes(&data) {
                Ok(event) => {
                    self.bus.do_send(event.clone());
                    self.sent_events.insert(event.into());
                }
                Err(err) => {
                    error!(error=?err, "Could not create EnclaveEvent from GossipData Bytes!")
                }
            },
            _ => (),
        }
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
