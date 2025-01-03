use actix::prelude::*;
use anyhow::Result;
use events::EventBus;
use futures::StreamExt;
use libp2p::{
    connection_limits::{self, ConnectionLimits},
    gossipsub::{self, MessageId},
    identify::{self, Behaviour as IdentifyBehaviour},
    identity::Keypair,
    kad::{store::MemoryStore, Behaviour as KademliaBehaviour},
    mdns,
    swarm::{behaviour::toggle::Toggle, NetworkBehaviour, SwarmEvent},
    Swarm,
};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::{hash::DefaultHasher, io::Error, time::Duration};
use tokio::{select, sync::mpsc};
use tracing::{debug, info, trace, warn};

use crate::events::NetworkPeerCommand;
use crate::events::NetworkPeerEvent;

#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: KademliaBehaviour<MemoryStore>,
    pub connection_limits: connection_limits::Behaviour,
    pub mdns: Toggle<mdns::tokio::Behaviour>,
    pub identify: IdentifyBehaviour,
}

pub struct NetworkPeer {
    swarm: Swarm<NodeBehaviour>,
    net_bus: Addr<EventBus<NetworkPeerEvent>>,
    cmd_rx: mpsc::Receiver<NetworkPeerCommand>,
}

impl NetworkPeer {
    pub fn new(
        id: &Keypair,
        enable_mdns: bool,
        net_bus: Addr<EventBus<NetworkPeerEvent>>,
        cmd_rx: mpsc::Receiver<NetworkPeerCommand>,
    ) -> Result<Self> {
        let swarm = libp2p::SwarmBuilder::with_existing_identity(id.clone())
            .with_tokio()
            .with_quic()
            .with_behaviour(|key| create_mdns_kad_behaviour(enable_mdns, key))
            .unwrap()
            .build();

        Ok(Self { swarm, net_bus, cmd_rx })
    }

    pub fn listen_on(&mut self, port: u16) -> Result<()> {
        self.swarm.listen_on(format!("/ip4/0.0.0.0/udp/{}/quic-v1", port).parse()?)?;
        Ok(())
    }

    pub fn subscribe(&mut self, topic: &gossipsub::IdentTopic) -> Result<()> {
        self.swarm.behaviour_mut().gossipsub.subscribe(topic)?;
        Ok(())
    }

    fn process_command(
        &mut self,
        command: NetworkPeerCommand,
    ) {
        match command {
            NetworkPeerCommand::GossipPublish {
                data,
                topic,
                correlation_id,
            } => {
                let gossipsub_behaviour = &mut self.swarm.behaviour_mut().gossipsub;
                match gossipsub_behaviour.publish(gossipsub::IdentTopic::new(topic), data) {
                    Ok(message_id) => {
                        self.net_bus.do_send(NetworkPeerEvent::GossipPublished {
                            correlation_id,
                            message_id,
                        });
                    }
                    Err(e) => {
                        warn!(error=?e, "Could not publish to swarm. Retrying...");
                        self.net_bus.do_send(NetworkPeerEvent::GossipPublishError {
                            correlation_id,
                            error: Arc::new(e),
                        });
                    }
                }
            }
            NetworkPeerCommand::Dial(multi) => {
                info!("DIAL: {:?}", multi);
                let connection_id = multi.connection_id();
                match self.swarm.dial(multi) {
                    Ok(v) => {
                        info!("Dial returned {:?}", v);
                    }
                    Err(error) => {
                        info!("Dialing error! {}", error);
                        self.net_bus.do_send(NetworkPeerEvent::DialError {
                            connection_id,
                            error: error.into(),
                        });
                    }
                }
            }
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        loop {
            select! {
                cmd = self.cmd_rx.recv() => {
                    if let Some(cmd) = cmd {
                        self.process_command(cmd);
                    }
                }
                event = self.swarm.select_next_some() => {
                    process_swarm_event(&mut self.swarm, &self.net_bus, event).await?
                }
            }
        }
    }
}

/// Create the libp2p behaviour
fn create_mdns_kad_behaviour(
    enable_mdns: bool,
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
        MessageId::from(s.finish().to_string())
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
        kademlia: KademliaBehaviour::new(
            key.public().to_peer_id(),
            MemoryStore::new(key.public().to_peer_id()),
        ),
        mdns,
        connection_limits,
        identify: identify_config,
    })
}

/// Process all swarm events
async fn process_swarm_event(
    swarm: &mut Swarm<NodeBehaviour>,
    net_bus: &Addr<EventBus<NetworkPeerEvent>>,
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

            info!("Added address to kademlia {}", remote_addr);
            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
            info!("Added peer to gossipsub {}", remote_addr);
            net_bus.do_send(NetworkPeerEvent::ConnectionEstablished { connection_id });
        }

        SwarmEvent::OutgoingConnectionError {
            peer_id,
            error,
            connection_id,
        } => {
            info!("Failed to dial {peer_id:?}: {error}");
            net_bus.do_send(NetworkPeerEvent::OutgoingConnectionError {
                connection_id,
                error: Arc::new(error),
            });
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
                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
            }
        }

        SwarmEvent::Behaviour(NodeBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
            for (peer_id, _multiaddr) in list {
                trace!("mDNS discover peer has expired: {peer_id}");
                swarm
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
            net_bus.do_send(NetworkPeerEvent::GossipData(message.data));
        }
        SwarmEvent::NewListenAddr { address, .. } => {
            warn!("Local node is listening on {address}");
        }
        _ => {}
    };
    Ok(())
}
