use crate::{
    dialer::{DialPeers, DialerActor, SetNetworkPeer},
    events::{NetworkPeerCommand, NetworkPeerEvent},
    network_manager::NetworkManager,
};
use actix::prelude::*;
use anyhow::Result;
use dev::AsyncContext;
use futures::task::noop_waker_ref;
use libp2p::{
    connection_limits::{self, ConnectionLimits},
    gossipsub::{self, IdentTopic, MessageId},
    identify::{self, Behaviour as IdentifyBehaviour},
    identity::Keypair,
    kad::{store::MemoryStore, Behaviour as KademliaBehaviour},
    mdns,
    swarm::{behaviour::toggle::Toggle, NetworkBehaviour, SwarmEvent},
    Swarm,
};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    io::Error,
    pin::Pin,
    sync::Arc,
    task::{Context as TaskContext, Poll},
    time::Duration,
};
use tracing::{debug, error, info, trace, warn};

// Actor Messages
#[derive(Message)]
#[rtype(result = "Result<()>")]
pub struct SetNetworkManager(pub Recipient<NetworkPeerEvent>);

#[derive(Message)]
#[rtype(result = "Result<()>")]
pub struct StartNetwork;

#[derive(Message)]
#[rtype(result = "Result<()>")]
pub struct SubscribeTopic(pub String);

#[derive(Message)]
#[rtype(result = "Result<()>")]
pub struct AddPeers(pub Vec<String>);

#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    gossipsub: gossipsub::Behaviour,
    kademlia: KademliaBehaviour<MemoryStore>,
    connection_limits: connection_limits::Behaviour,
    mdns: Toggle<mdns::tokio::Behaviour>,
    identify: IdentifyBehaviour,
}

pub struct NetworkPeer {
    swarm: Swarm<NodeBehaviour>,
    peers: Vec<String>,
    quic_port: Option<u16>,
    dialer: Option<Addr<DialerActor>>,
    mgr: Option<Recipient<NetworkPeerEvent>>,
}

impl NetworkPeer {
    pub fn new(
        id: &Keypair,
        initial_peers: Vec<String>,
        quic_port: Option<u16>,
        enable_mdns: bool,
    ) -> Self {
        let swarm = libp2p::SwarmBuilder::with_existing_identity(id.clone())
            .with_tokio()
            .with_quic()
            .with_behaviour(|key| create_mdns_kad_behaviour(enable_mdns, key))
            .unwrap()
            .build();

        let dialer = DialerActor::new();

        Self {
            swarm,
            peers: initial_peers,
            quic_port,
            mgr: None,
            dialer: Some(dialer),
        }
    }

    pub fn setup(
        id: &Keypair,
        initial_peers: Vec<String>,
        quic_port: Option<u16>,
        enable_mdns: bool,
    ) -> Addr<Self> {
        let peer = Self::new(id, initial_peers, quic_port, enable_mdns).start();
        peer
    }

    fn begin_swarm_polling(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(Duration::from_millis(50), |actor, _ctx| {
            let mut pinned = Pin::new(&mut actor.swarm);
            let waker = noop_waker_ref();
            let mut async_ctx = TaskContext::from_waker(waker);
            let mut events = Vec::new();
            loop {
                match pinned.as_mut().poll_next(&mut async_ctx) {
                    Poll::Ready(Some(event)) => events.push(event),
                    Poll::Ready(None) => {
                        break;
                    }
                    Poll::Pending => {
                        break;
                    }
                }
            }
            for evt in events {
                if let Err(e) = actor.handle_swarm_event(evt) {
                    error!("Error handling swarm event: {}", e);
                }
            }
        });
    }

    fn handle_swarm_event(&mut self, event: SwarmEvent<NodeBehaviourEvent>) -> Result<()> {
        if let Some(mgr) = &mut self.mgr {
            match event {
                SwarmEvent::ConnectionEstablished {
                    peer_id,
                    endpoint,
                    connection_id,
                    ..
                } => {
                    info!("Connected to {peer_id}");
                    let remote_addr = endpoint.get_remote_address().clone();
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, remote_addr.clone());

                    info!("Added address to kademlia {}", remote_addr);
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .add_explicit_peer(&peer_id);
                    info!("Added peer to gossipsub {}", remote_addr);
                    mgr.do_send(NetworkPeerEvent::ConnectionEstablished { connection_id });
                    if let Some(dialer) = &mut self.dialer {
                        dialer.do_send(NetworkPeerEvent::ConnectionEstablished { connection_id });
                    }
                }

                SwarmEvent::OutgoingConnectionError {
                    peer_id,
                    error,
                    connection_id,
                } => {
                    info!("Failed to dial {peer_id:?}: {error}");
                    if let Some(dialer) = &mut self.dialer {
                        dialer.do_send(NetworkPeerEvent::OutgoingConnectionError {
                            connection_id,
                            error: Arc::new(error),
                        });
                    }
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
                        self.swarm
                            .behaviour_mut()
                            .gossipsub
                            .add_explicit_peer(&peer_id);
                    }
                }

                SwarmEvent::Behaviour(NodeBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        trace!("mDNS discover peer has expired: {peer_id}");
                        self.swarm
                            .behaviour_mut()
                            .gossipsub
                            .remove_explicit_peer(&peer_id);
                    }
                }

                SwarmEvent::Behaviour(NodeBehaviourEvent::Gossipsub(
                    gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message_id: id,
                        message,
                    },
                )) => {
                    info!("Got message with id: {id} from peer: {peer_id}",);
                    mgr.do_send(NetworkPeerEvent::GossipData(message.data));
                }

                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Local node is listening on {address}");
                }
                _ => {}
            };
        }
        Ok(())
    }
}

impl Actor for NetworkPeer {
    type Context = Context<Self>;
}

impl Handler<StartNetwork> for NetworkPeer {
    type Result = Result<()>;

    fn handle(&mut self, _: StartNetwork, ctx: &mut Self::Context) -> Self::Result {
        let addr = match self.quic_port {
            Some(port) => format!("/ip4/0.0.0.0/udp/{}/quic-v1", port),
            None => "/ip4/0.0.0.0/udp/0/quic-v1".to_string(),
        };

        info!("Requesting node.listen_on('{}') ", addr);
        match self.swarm.listen_on(addr.parse().unwrap()) {
            Ok(i) => {
                info!("Started Listening with ID {}", i);
            }
            Err(e) => {
                error!("Error listening on {}: {}", addr, e);
                return Err(anyhow::anyhow!("Error listening on {}: {}", addr, e));
            }
        }

        if let Some(dialer) = &mut self.dialer {
            let dialer_clone = dialer.clone();
            let peers = self.peers.clone();
            let address = ctx.address();

            ctx.spawn(
                async move {
                    if let Err(e) = dialer_clone.send(SetNetworkPeer(address)).await {
                        error!("Error setting network peer: {}", e);
                        return;
                    }
                    if !peers.is_empty() {
                        dialer_clone.do_send(DialPeers(peers));
                    }
                }
                .into_actor(self),
            );

            self.begin_swarm_polling(ctx);
        }
        Ok(())
    }
}

impl Handler<SetNetworkManager> for NetworkPeer {
    type Result = Result<()>;

    fn handle(&mut self, msg: SetNetworkManager, _: &mut Self::Context) -> Self::Result {
        self.mgr = Some(msg.0);
        Ok(())
    }
}

impl Handler<SubscribeTopic> for NetworkPeer {
    type Result = Result<()>;

    fn handle(&mut self, msg: SubscribeTopic, _: &mut Self::Context) -> Self::Result {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&IdentTopic::new(msg.0))?;
        Ok(())
    }
}

impl Handler<AddPeers> for NetworkPeer {
    type Result = Result<()>;

    fn handle(&mut self, msg: AddPeers, _: &mut Self::Context) -> Self::Result {
        // Store new peers
        self.peers.extend(msg.0.clone());

        // Attempt to dial them immediately
        if let Some(dialer) = &mut self.dialer {
            dialer.do_send(DialPeers(msg.0));
        }
        Ok(())
    }
}

impl Handler<NetworkPeerCommand> for NetworkPeer {
    type Result = ();

    fn handle(&mut self, msg: NetworkPeerCommand, _: &mut Self::Context) {
        match msg {
            NetworkPeerCommand::Dial(opts) => {
                let conn_id = opts.connection_id();
                info!("Dialing {:?}", conn_id);
                match self.swarm.dial(opts) {
                    Ok(_) => {
                        info!("Successfully dialed {:?}", conn_id);
                    }
                    Err(error) => {
                        info!("Dialing error! {}", error);
                        if let Some(dialer) = &mut self.dialer {
                            dialer.do_send(NetworkPeerEvent::DialError {
                                error: Arc::new(error),
                                connection_id: conn_id,
                            });
                        }
                    }
                }
            }
            NetworkPeerCommand::GossipPublish {
                topic,
                data,
                correlation_id,
            } => {
                let gossipsub = &mut self.swarm.behaviour_mut().gossipsub;
                if let Some(mgr) = &mut self.mgr {
                    match gossipsub.publish(IdentTopic::new(topic), data) {
                        Ok(message_id) => {
                            mgr.do_send(NetworkPeerEvent::GossipPublished {
                                correlation_id: correlation_id,
                                message_id,
                            });
                        }
                        Err(e) => {
                            warn!(error=?e, "Could not publish to swarm");
                            if let Some(mgr) = &mut self.mgr {
                                mgr.do_send(NetworkPeerEvent::GossipPublishError {
                                    correlation_id: correlation_id,
                                    error: Arc::new(e),
                                });
                            }
                        }
                    }
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
