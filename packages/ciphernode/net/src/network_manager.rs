use crate::{
    correlation_id::CorrelationId,
    events::{NetworkPeerCommand, NetworkPeerEvent},
    network_peer::{SetNetworkManager, NetworkPeer, SubscribeTopic, StartListening},
};
/// Actor for connecting to an libp2p client via it's mpsc channel interface
/// This Actor should be responsible for
use actix::prelude::*;
use anyhow::{bail, Result};
use crypto::Cipher;
use data::Repository;
use events::{EnclaveEvent, EventBus, EventId, Subscribe};
use libp2p::identity::ed25519;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{error, info, instrument, trace};

/// NetworkManager Actor converts between EventBus events and Libp2p events forwarding them to a
/// NetworkPeer for propagation over the p2p network
pub struct NetworkManager {
    bus: Addr<EventBus>,
    peer: Addr<NetworkPeer>,
    sent_events: HashSet<EventId>,
    topic: String,
}

impl Actor for NetworkManager {
    type Context = Context<Self>;
}

impl NetworkManager {
    /// Create a new NetworkManager actor
    pub fn new(bus: Addr<EventBus>, peer: Addr<NetworkPeer>, topic: &str) -> Self {
        Self {
            bus,
            peer,
            sent_events: HashSet::new(),
            topic: topic.to_string(),
        }
    }

    pub fn setup(bus: Addr<EventBus>, peer: Addr<NetworkPeer>, topic: &str) -> Addr<Self> {
        let addr = NetworkManager::new(bus.clone(), peer.clone(), topic).start();

        // Listen on all events
        bus.do_send(Subscribe {
            event_type: String::from("*"),
            listener: addr.clone().recipient(),
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
    ) -> Result<(Addr<Self>, String)> {
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
        let peer = NetworkPeer::setup(&keypair, peers, enable_mdns);
        peer.send(StartListening(Some(quic_port)));
        peer.send(SubscribeTopic(topic.to_string()));

        // Setup and start network manager
        let p2p_addr = NetworkManager::setup(bus, peer.clone(), topic);
        peer.send(SetNetworkManager(p2p_addr.clone()));

        Ok((p2p_addr, keypair.public().to_peer_id().to_string()))
    }
}

impl Handler<NetworkPeerEvent> for NetworkManager {
    type Result = ();
    fn handle(&mut self, event: NetworkPeerEvent, _: &mut Self::Context) -> Self::Result {
        match event {
            NetworkPeerEvent::GossipData(data) => match EnclaveEvent::from_bytes(&data) {
                Ok(event) => {
                    self.bus.do_send(event.clone());
                    self.sent_events.insert(event.into());
                }
                Err(error) => {
                    error!(error=?error, "Could not create EnclaveEvent from Libp2p Bytes!");
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
        let evt = event.clone();
        let topic = self.topic.clone();
        let peer = self.peer.clone();
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
                    peer.send(NetworkPeerCommand::GossipPublish {
                        topic,
                        data,
                        correlation_id: CorrelationId::new(),
                    });
                }
                Err(error) => {
                    error!(error=?error, "Could not convert event to bytes for serialization!")
                }
            }
        })
    }
}
