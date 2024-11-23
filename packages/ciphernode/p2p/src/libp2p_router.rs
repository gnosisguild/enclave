use anyhow::Result;
use libp2p::connection_limits::ConnectionLimits;
use libp2p::{
    connection_limits, futures::StreamExt, gossipsub, identify::Behaviour as IdentifyBehaviour,
    identity, kad::store::MemoryStore, kad::Behaviour as KademliaBehaviour,
    swarm::NetworkBehaviour, swarm::SwarmEvent,
};
use libp2p::{identify, mdns, Multiaddr};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::{io, select};
use tracing::{debug, error, info, trace, warn};

#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    gossipsub: gossipsub::Behaviour,
    kademlia: KademliaBehaviour<MemoryStore>,
    connection_limits: connection_limits::Behaviour,
    mdns: mdns::tokio::Behaviour,
    identify: IdentifyBehaviour,
}

pub struct EnclaveRouter {
    pub identity: Option<identity::Keypair>,
    pub gossipsub_config: gossipsub::Config,
    pub swarm: Option<libp2p::Swarm<NodeBehaviour>>,
    pub topic: Option<gossipsub::IdentTopic>,
    udp_port: Option<u16>,
    evt_tx: Sender<Vec<u8>>,
    cmd_rx: Receiver<Vec<u8>>,
}

impl EnclaveRouter {
    pub fn new() -> Result<(Self, Sender<Vec<u8>>, Receiver<Vec<u8>>)> {
        let (evt_tx, evt_rx) = channel(100); // TODO : tune this param
        let (cmd_tx, cmd_rx) = channel(100); // TODO : tune this param
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };
        // TODO: Allow for config inputs to new()
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .message_id_fn(message_id_fn)
            .build()
            .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?;

        Ok((
            Self {
                identity: None,
                gossipsub_config,
                swarm: None,
                topic: None,
                evt_tx,
                cmd_rx,
                udp_port: None,
            },
            cmd_tx,
            evt_rx,
        ))
    }

    pub async fn dial_peer(&mut self, addr: &str) -> Result<()> {
        let multiaddr: Multiaddr = addr.parse()?;
        let swarm = self
            .swarm
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Swarm not initialized"))?;
        swarm.dial(multiaddr.clone()).map_err(|e| {
            error!("Failed to dial {}: {}", multiaddr, e);
            anyhow::anyhow!("Failed to dial peer: {}", e)
        })?;
        info!("Dailed peer successfully at {}", multiaddr);
        Ok(())
    }

    // TODO: create proper builder instead of this fudgy version
    // get working for now then refactor
    pub fn with_identity(&mut self, keypair: &identity::Keypair) -> &mut Self {
        self.identity = Some(keypair.clone());
        self
    }

    pub fn with_udp_port(&mut self, port: u16) -> &mut Self {
        self.udp_port = Some(port);
        self
    }

    pub fn connect_swarm(&mut self) -> Result<&mut Self> {
        let connection_limits = connection_limits::Behaviour::new(ConnectionLimits::default());
        let identify_config = IdentifyBehaviour::new(
            identify::Config::new(
                "/kad/0.1.0".into(),
                self.identity.as_ref().unwrap().public(),
            )
            .with_interval(Duration::from_secs(60)), // do this so we can get timeouts for dropped WebRTC connections
        );
        let swarm = self
            .identity
            .clone()
            .map_or_else(
                || libp2p::SwarmBuilder::with_new_identity(),
                |id| libp2p::SwarmBuilder::with_existing_identity(id),
            )
            .with_tokio()
            .with_quic()
            .with_behaviour(|key| {
                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    self.gossipsub_config.clone(),
                )
                .expect("Failed to create gossipsub behavior");

                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?;
                Ok(NodeBehaviour {
                    gossipsub,
                    kademlia: KademliaBehaviour::new(
                        key.public().to_peer_id(),
                        MemoryStore::new(key.public().to_peer_id()),
                    ),
                    mdns,
                    connection_limits,
                    identify: identify_config,
                })
            })?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();
        self.swarm = Some(swarm);
        Ok(self)
    }

    pub fn join_topic(&mut self, topic_name: &str) -> Result<&mut Self> {
        let topic = gossipsub::IdentTopic::new(topic_name);
        self.topic = Some(topic.clone());
        self.swarm
            .as_mut()
            .unwrap()
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic)?;
        Ok(self)
    }

    /// Listen on the default multiaddr
    pub async fn start(&mut self, peers: Vec<String>) -> Result<()> {
        let addr = match self.udp_port {
            Some(port) => format!("/ip4/0.0.0.0/udp/{}/quic-v1", port),
            None => "/ip4/0.0.0.0/udp/0/quic-v1".to_string(),
        };
        info!("Requesting node.listen_on('{}')", addr);
        self.swarm.as_mut().unwrap().listen_on(addr.parse()?)?;

        info!("Peers to dial: {:?}", peers);
        for addr in peers {
            self.dial_peer(&addr).await?;
        }

        loop {
            select! {
                Some(line) = self.cmd_rx.recv() => {
                    if let Err(e) = self.swarm.as_mut().unwrap()
                        .behaviour_mut().gossipsub
                        .publish(self.topic.as_mut().unwrap().clone(), line) {
                        error!(error=?e, "Error publishing line to swarm");
                    }
                }

                event = self.swarm.as_mut().unwrap().select_next_some() => match event {

                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    info!("Connected to {peer_id}");
                }

                SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                    warn!("Failed to dial {peer_id:?}: {error}");
                }

                SwarmEvent::IncomingConnectionError { error, .. } => {
                    warn!("{:#}", anyhow::Error::from(error))
                }

                SwarmEvent::Behaviour(NodeBehaviourEvent::Kademlia(e)) => {
                    debug!("Kademlia event: {:?}", e);
                }

                SwarmEvent::Behaviour(NodeBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        trace!("mDNS discovered a new peer: {peer_id}");
                        self.swarm.as_mut().unwrap().behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },

                SwarmEvent::Behaviour(NodeBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        trace!("mDNS discover peer has expired: {peer_id}");
                        self.swarm.as_mut().unwrap().behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                },

                SwarmEvent::Behaviour(NodeBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) => {
                    trace!(
                        "Got message with id: {id} from peer: {peer_id}",
                    );
                    trace!("{:?}", message);
                        self.evt_tx.send(message.data).await?;
                    },
                    SwarmEvent::NewListenAddr { address, .. } => {
                        warn!("Local node is listening on {address}");
                    }
                    _ => {}
                }
            }
        }
    }
}
