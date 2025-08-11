// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use libp2p::{
    connection_limits::{self, ConnectionLimits},
    futures::StreamExt,
    gossipsub,
    identify::{self, Behaviour as IdentifyBehaviour},
    identity::Keypair,
    kad::{store::MemoryStore, Behaviour as KademliaBehaviour},
    swarm::{NetworkBehaviour, SwarmEvent},
    Swarm,
};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::{hash::DefaultHasher, io::Error, time::Duration};
use tokio::{select, sync::broadcast, sync::mpsc};
use tracing::{debug, info, trace, warn};

use crate::dialer::dial_peers;
use crate::events::NetCommand;
use crate::events::NetEvent;

#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    gossipsub: gossipsub::Behaviour,
    kademlia: KademliaBehaviour<MemoryStore>,
    connection_limits: connection_limits::Behaviour,
    identify: IdentifyBehaviour,
}

/// Manage the peer to peer connection. This struct wraps a libp2p Swarm and enables communication
/// with it using channels.
pub struct NetInterface {
    /// The Libp2p Swarm instance
    swarm: Swarm<NodeBehaviour>,
    /// A list of peers to automatically dial
    peers: Vec<String>,
    /// The UDP port that the peer listens to over QUIC
    udp_port: Option<u16>,
    /// The gossipsub topic that the peer should listen on
    topic: gossipsub::IdentTopic,
    /// Broadcast channel to report NetEvents to listeners
    event_tx: broadcast::Sender<NetEvent>,
    /// Transmission channel to send NetCommands to the NetInterface
    cmd_tx: mpsc::Sender<NetCommand>,
    /// Local receiver to process NetCommands from
    cmd_rx: mpsc::Receiver<NetCommand>,
}

impl NetInterface {
    pub fn new(
        id: &Keypair,
        peers: Vec<String>,
        udp_port: Option<u16>,
        topic: &str,
    ) -> Result<Self> {
        let (event_tx, _) = broadcast::channel(100); // TODO : tune this param
        let (cmd_tx, cmd_rx) = mpsc::channel(100); // TODO : tune this param

        let swarm = libp2p::SwarmBuilder::with_existing_identity(id.clone())
            .with_tokio()
            .with_quic()
            .with_behaviour(|key| create_kad_behaviour(key))?
            .build();

        // TODO: Use topics to manage network traffic instead of just using a single topic
        let topic = gossipsub::IdentTopic::new(topic);

        Ok(Self {
            swarm,
            peers,
            udp_port,
            topic,
            event_tx,
            cmd_tx,
            cmd_rx,
        })
    }

    pub fn rx(&mut self) -> broadcast::Receiver<NetEvent> {
        self.event_tx.subscribe()
    }

    pub fn tx(&self) -> mpsc::Sender<NetCommand> {
        self.cmd_tx.clone()
    }

    pub async fn start(&mut self) -> Result<()> {
        let event_tx = self.event_tx.clone();
        let cmd_tx = self.cmd_tx.clone();
        let cmd_rx = &mut self.cmd_rx;

        // Subscribe to topic
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.topic)?;

        // Listen on the quic port
        let addr = match self.udp_port {
            Some(port) => format!("/ip4/0.0.0.0/udp/{}/quic-v1", port),
            None => "/ip4/0.0.0.0/udp/0/quic-v1".to_string(),
        };

        trace!("Requesting node.listen_on('{}')", addr);
        self.swarm.listen_on(addr.parse()?)?;

        trace!("Peers to dial: {:?}", self.peers);
        tokio::spawn({
            let event_tx = event_tx.clone();
            let peers = self.peers.clone();
            async move {
                dial_peers(&cmd_tx, &event_tx, &peers).await?;

                return anyhow::Ok(());
            }
        });

        loop {
            select! {
                 // Process commands
                Some(command) = cmd_rx.recv() => {
                    match command {
                        NetCommand::GossipPublish { data, topic, correlation_id } => {
                            let gossipsub_behaviour = &mut self.swarm.behaviour_mut().gossipsub;
                            match gossipsub_behaviour
                                .publish(gossipsub::IdentTopic::new(topic), data) {
                                Ok(message_id) => {
                                    event_tx.send(NetEvent::GossipPublished { correlation_id, message_id })?;
                                },
                                Err(e) => {
                                    warn!(error=?e, "Could not publish to swarm. Retrying...");
                                    event_tx.send(NetEvent::GossipPublishError { correlation_id, error: Arc::new(e) })?;
                                }
                            }
                        },
                        NetCommand::Dial(multi) => {
                            trace!("DIAL: {:?}", multi);
                            match self.swarm.dial(multi) {
                                Ok(v) => trace!("Dial returned {:?}", v),
                                Err(error) => {
                                    warn!("Dialing error! {}", error);
                                    event_tx.send(NetEvent::DialError { error: error.into() })?;
                                }
                            }
                        },
                        NetCommand::PublishDocument { meta:_, value:_, cid:_ } => todo!(),
                        NetCommand::FetchDocument { cid:_ } => todo!(),

                    }
                }
                // Process events
                event = self.swarm.select_next_some() =>  {
                    process_swarm_event(&mut self.swarm, &event_tx, event).await?
                }
            }
        }
    }
}

/// Create the libp2p behaviour
fn create_kad_behaviour(
    key: &Keypair,
) -> std::result::Result<NodeBehaviour, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let connection_limits = connection_limits::Behaviour::new(ConnectionLimits::default());
    let identify_config = IdentifyBehaviour::new(
        identify::Config::new("/kad/0.1.0".into(), key.public())
            .with_interval(Duration::from_secs(60)),
    );

    let message_id_fn = |message: &gossipsub::Message| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        gossipsub::MessageId::from(s.finish().to_string())
    };

    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .build()
        .map_err(|msg| Error::new(std::io::ErrorKind::Other, msg))?;

    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(key.clone()),
        gossipsub_config,
    )?;

    Ok(NodeBehaviour {
        gossipsub,
        kademlia: KademliaBehaviour::new(
            key.public().to_peer_id(),
            MemoryStore::new(key.public().to_peer_id()),
        ),
        connection_limits,
        identify: identify_config,
    })
}

/// Process all swarm events
async fn process_swarm_event(
    swarm: &mut Swarm<NodeBehaviour>,
    event_tx: &broadcast::Sender<NetEvent>,
    event: SwarmEvent<NodeBehaviourEvent>,
) -> Result<()> {
    match event {
        SwarmEvent::ConnectionEstablished {
            peer_id,
            endpoint,
            connection_id,
            ..
        } => {
            info!("Connected to {peer_id}");
            let remote_addr = endpoint.get_remote_address().clone();
            swarm
                .behaviour_mut()
                .kademlia
                .add_address(&peer_id, remote_addr.clone());

            trace!("Added address to kademlia {}", remote_addr);
            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
            trace!("Added peer to gossipsub {}", remote_addr);
            event_tx.send(NetEvent::ConnectionEstablished { connection_id })?;
        }

        SwarmEvent::OutgoingConnectionError {
            peer_id,
            error,
            connection_id,
        } => {
            warn!("Failed to dial {peer_id:?}: {error}");
            event_tx.send(NetEvent::OutgoingConnectionError {
                connection_id,
                error: Arc::new(error),
            })?;
        }

        SwarmEvent::IncomingConnectionError { error, .. } => {
            warn!("{:#}", anyhow::Error::from(error))
        }

        SwarmEvent::Behaviour(NodeBehaviourEvent::Kademlia(e)) => {
            debug!("Kademlia event: {:?}", e);
        }

        SwarmEvent::Behaviour(NodeBehaviourEvent::Gossipsub(gossipsub::Event::Message {
            propagation_source: peer_id,
            message_id: id,
            message,
        })) => {
            trace!("Got message with id: {id} from peer: {peer_id}");
            // TODO: attempt to deserialize a DocumentPublishedPayload from the message.data if
            // that works then send NetEvent::DocumentPublishedNotification over the event_tx otherwise send
            // GossipData
            event_tx.send(NetEvent::GossipData(message.data))?;
        }
        SwarmEvent::NewListenAddr { address, .. } => {
            trace!("Local node is listening on {address}");
        }
        _ => {}
    };
    Ok(())
}
