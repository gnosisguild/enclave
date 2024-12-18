use anyhow::Result;
use enclave_core::{EnclaveEvent, NetworkReady};
use libp2p::{
    connection_limits::{self, ConnectionLimits},
    futures::StreamExt,
    gossipsub,
    identify::{self, Behaviour as IdentifyBehaviour},
    identity::Keypair,
    kad::{self, store::MemoryStore, Behaviour as KademliaBehaviour},
    mdns,
    swarm::{behaviour::toggle::Toggle, NetworkBehaviour, SwarmEvent},
    Multiaddr, StreamProtocol, Swarm,
};
use std::{hash::DefaultHasher, io::Error, time::Duration};
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};
use tokio::{
    select,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
};
use tracing::{debug, error, info, trace, warn};

use crate::retry::{RetryConfig, RetryHandler};

#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    gossipsub: gossipsub::Behaviour,
    kademlia: KademliaBehaviour<MemoryStore>,
    connection_limits: connection_limits::Behaviour,
    mdns: Toggle<mdns::tokio::Behaviour>,
    identify: IdentifyBehaviour,
}

pub struct NetworkPeer {
    swarm: Arc<Mutex<Swarm<NodeBehaviour>>>,
    peers: Vec<String>,
    udp_port: Option<u16>,
    topic: gossipsub::IdentTopic,
    to_bus_tx: Sender<Vec<u8>>,             // to event bus
    from_net_rx: Option<Receiver<Vec<u8>>>, // from network
    to_net_tx: Sender<Vec<u8>>,             // to network
    from_bus_rx: Receiver<Vec<u8>>,         // from event bus
}

impl NetworkPeer {
    pub fn new(
        id: &Keypair,
        peers: Vec<String>,
        udp_port: Option<u16>,
        topic: &str,
        enable_mdns: bool,
    ) -> Result<Self> {
        let (to_bus_tx, from_net_rx) = channel(100); // TODO : tune this param
        let (to_net_tx, from_bus_rx) = channel(100); // TODO : tune this param

        let swarm = Arc::new(Mutex::new(
            libp2p::SwarmBuilder::with_existing_identity(id.clone())
                .with_tokio()
                .with_quic()
                .with_behaviour(|key| create_mdns_kad_behaviour(enable_mdns, key))?
                .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
                .build(),
        ));

        // TODO: Use topics to manage network traffic instead of just using a single topic
        let topic = gossipsub::IdentTopic::new(topic);

        Ok(Self {
            swarm,
            peers,
            udp_port,
            topic,
            to_bus_tx,
            from_net_rx: Some(from_net_rx),
            to_net_tx,
            from_bus_rx,
        })
    }

    pub fn rx(&mut self) -> Option<Receiver<Vec<u8>>> {
        self.from_net_rx.take()
    }

    pub fn tx(&self) -> Sender<Vec<u8>> {
        self.to_net_tx.clone()
    }

    pub async fn start(&mut self) -> Result<()> {
        let addr = match self.udp_port {
            Some(port) => format!("/ip4/0.0.0.0/udp/{}/quic-v1", port),
            None => "/ip4/0.0.0.0/udp/0/quic-v1".to_string(),
        };
        info!("Starting network actor");
        info!("Requesting node.listen_on('{}')", addr);

        self.swarm
            .lock()
            .await
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.topic)?;
        // self.swarm.lock().await.listen_on(addr.parse()?)?;

        if let Err(e) = self.swarm.lock().await.listen_on(addr.parse()?) {
            warn!("Failed to listen on {addr}: {e}");
        }

        tokio::spawn({
            let peers = self.peers.clone();
            let swarm = self.swarm.clone();
            async move {
                info!("Peers to dial: {:?}", &peers);
                for addr in peers.clone() {
                    info!("Parsing multiaddr: {}", addr);
                    let multiaddr: Multiaddr = addr.parse()?;
                    dial_peer(&swarm, &multiaddr).await?;
                }
                info!("Finished dialing: {:?}", &peers);

                anyhow::Ok(())
            }
        });

        loop {
            let swarm = &mut self.swarm.lock().await;
            select! {
                Some(line) = self.from_bus_rx.recv() => {
                    if let Err(e) = swarm
                        .behaviour_mut().gossipsub
                        .publish(self.topic.clone(), line) {
                        error!(error=?e, "Error publishing line to swarm");
                    }
                }

                event = swarm.select_next_some() =>  {
                    process_swarm_event(&self.swarm, &mut self.to_bus_tx, event).await?
                }
            }
        }
    }
}

fn create_mdns_kad_behaviour(
    enable_mdns: bool,
    key: &Keypair,
) -> std::result::Result<NodeBehaviour, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let connection_limits = connection_limits::Behaviour::new(ConnectionLimits::default());
    let identify_config = IdentifyBehaviour::new(
        identify::Config::new("/kad/1.0.0".into(), key.public())
            .with_interval(Duration::from_secs(60)),
    );

    let kad_config = kad::Config::new(StreamProtocol::new("/kad/1.0.0"));

    let message_id_fn = |message: &gossipsub::Message| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        gossipsub::MessageId::from(s.finish().to_string())
    };

    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .mesh_n(3)
        .mesh_n_low(2)
        .mesh_outbound_min(1)
        .heartbeat_interval(Duration::from_secs(10))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .build()
        .map_err(|msg| Error::new(std::io::ErrorKind::Other, msg))?;

    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(key.clone()),
        gossipsub_config,
    )?;

    let mdns = if enable_mdns {
        Toggle::from(Some(mdns::tokio::Behaviour::new(
            mdns::Config::default(),
            key.public().to_peer_id(),
        )?))
    } else {
        Toggle::from(None)
    };

    Ok(NodeBehaviour {
        gossipsub,
        kademlia: KademliaBehaviour::with_config(
            key.public().to_peer_id(),
            MemoryStore::new(key.public().to_peer_id()),
            kad_config,
        ),
        mdns,
        connection_limits,
        identify: identify_config,
    })
}

pub async fn dial_peer(swarm: &Arc<Mutex<Swarm<NodeBehaviour>>>, addr: &Multiaddr) -> Result<()> {
    let retry_handler = RetryHandler::new(
        RetryConfig::default()
            .with_max_retries(5)
            .with_initial_delay(Duration::from_secs(1)),
    );

    retry_handler
        .retry(
            || async {
                let mut swarm = swarm.lock().await;
                swarm.dial(addr.clone())?;
                anyhow::Ok(())
            },
            &format!("Dialing peer {}", addr), // This is passed to logging
        )
        .await
}

async fn process_swarm_event(
    swarm: &Arc<Mutex<Swarm<NodeBehaviour>>>,
    to_bus_tx: &mut Sender<Vec<u8>>,
    event: SwarmEvent<NodeBehaviourEvent>,
) -> Result<()> {
    match event {
        SwarmEvent::ConnectionEstablished {
            peer_id, endpoint, ..
        } => {
            info!("Connected to {peer_id}");
            let remote_addr = endpoint.get_remote_address().clone();
            let mut guard = swarm.lock().await;
            guard
                .behaviour_mut()
                .kademlia
                .add_address(&peer_id, remote_addr.clone());

            info!("Added address to kademlia {}", remote_addr);
            guard.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
            info!("Added peer to gossipsub {}", remote_addr);
            // Send NetworkReady event which will be deduped because it contains no unique data
            // This will allow other services to initialize based on network being ready
            to_bus_tx
                .send(EnclaveEvent::from(NetworkReady {}).to_bytes()?)
                .await?;
        }

        SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
            warn!("Failed to dial {peer_id:?}: {error}");
        }

        SwarmEvent::IncomingConnectionError { error, .. } => {
            warn!("{:#}", anyhow::Error::from(error))
        }

        SwarmEvent::Behaviour(NodeBehaviourEvent::Kademlia(kad::Event::InboundRequest {
            request,
        })) => {
            debug!("Inbound Kademlia request: {:?}", request);
        }

        SwarmEvent::Behaviour(NodeBehaviourEvent::Kademlia(e)) => {
            debug!("Other Kademlia event: {:?}", e);
        }

        SwarmEvent::Behaviour(NodeBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
            for (peer_id, _multiaddr) in list {
                trace!("mDNS discovered a new peer: {peer_id}");
                swarm
                    .lock()
                    .await
                    .behaviour_mut()
                    .gossipsub
                    .add_explicit_peer(&peer_id);
            }
        }

        SwarmEvent::Behaviour(NodeBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
            for (peer_id, _multiaddr) in list {
                trace!("mDNS discover peer has expired: {peer_id}");
                swarm
                    .lock()
                    .await
                    .behaviour_mut()
                    .gossipsub
                    .remove_explicit_peer(&peer_id);
            }
        }

        SwarmEvent::Behaviour(NodeBehaviourEvent::Gossipsub(gossipsub::Event::Message {
            propagation_source: peer_id,
            message_id: id,
            message,
        })) => {
            trace!("Got message with id: {id} from peer: {peer_id}",);
            trace!("{:?}", message);
            to_bus_tx.send(message.data).await?;
        }
        SwarmEvent::NewListenAddr { address, .. } => {
            warn!("Local node is listening on {address}");
        }
        _ => {}
    };
    Ok(())
}
