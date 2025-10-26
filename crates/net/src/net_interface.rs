// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::dialer::dial_peers;
use crate::events::{GossipData, NetCommand};
use crate::{events::NetEvent, Cid};
use anyhow::Result;
use e3_events::CorrelationId;
use e3_utils::ArcBytes;
use libp2p::gossipsub::TopicHash;
use libp2p::kad::{self, GetRecordOk, QueryId, QueryResult};
use libp2p::{
    connection_limits::{self, ConnectionLimits},
    futures::StreamExt,
    gossipsub,
    identify::{self, Behaviour as IdentifyBehaviour},
    identity::Keypair,
    kad::{store::MemoryStore, Behaviour as KademliaBehaviour, Quorum, Record, RecordKey},
    swarm::{dial_opts::DialOpts, NetworkBehaviour, SwarmEvent},
    Swarm,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{hash::DefaultHasher, io::Error, time::Duration};
use std::{
    hash::{Hash, Hasher},
    time::Instant,
};
use tokio::time::{sleep, timeout};
use tokio::{select, sync::broadcast, sync::mpsc};
use tracing::{debug, error, info, trace, warn};

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
        mesh_params: MeshParams,
    ) -> Result<Self> {
        println!("{:?}", mesh_params);
        let (event_tx, _) = broadcast::channel(100); // TODO : tune this param
        let (cmd_tx, cmd_rx) = mpsc::channel(100); // TODO : tune this param

        let swarm = libp2p::SwarmBuilder::with_existing_identity(id.clone())
            .with_tokio()
            .with_quic()
            .with_behaviour(|key| create_behaviour(key, mesh_params))?
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
        let mut correlator = Correlator::new();
        let shutdown_flag = Arc::new(AtomicBool::new(false));

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
                    if shutdown_flag.load(Ordering::Relaxed) {
                        continue; // Skip processing during shutdown
                    }
                    let result = match command {
                        NetCommand::GossipPublish { data, topic, correlation_id } =>
                            handle_gossip_publish(&mut self.swarm, &event_tx, data, topic, correlation_id),
                        NetCommand::Dial(multi) =>
                            handle_dial(&mut self.swarm, &event_tx, multi),
                        NetCommand::DhtPutRecord { correlation_id, key, expires, value } =>
                            handle_put_record(&mut self.swarm, &event_tx,&mut correlator, correlation_id, key, expires, value),
                        NetCommand::DhtGetRecord { correlation_id, key } =>
                            handle_get_record(&mut self.swarm, &mut correlator, correlation_id, key),
                        NetCommand::Shutdown => handle_shutdown(&mut self.swarm, &shutdown_flag)
                    };
                    match result {
                        Ok(_) => (),
                        Err(e) => error!("NetCommand Error: {e}")
                    }
                }

                // Process events
                event = self.swarm.select_next_some() =>  {
                    if shutdown_flag.load(Ordering::Relaxed){
                        continue; // Ignore new events during shutdown
                    }
                    process_swarm_event(&mut self.swarm, &event_tx, &mut correlator, event).await?
                }
            }
        }
    }
}

fn handle_gossip_publish(
    swarm: &mut Swarm<NodeBehaviour>,
    event_tx: &broadcast::Sender<NetEvent>,
    data: GossipData,
    topic: String,
    correlation_id: CorrelationId,
) -> Result<()> {
    let gossipsub_behaviour = &mut swarm.behaviour_mut().gossipsub;
    match gossipsub_behaviour.publish(gossipsub::IdentTopic::new(topic), data.to_bytes()?) {
        Ok(message_id) => {
            event_tx.send(NetEvent::GossipPublished {
                correlation_id,
                message_id,
            })?;
        }
        Err(e) => {
            warn!(error=?e, "Could not publish to swarm. Retrying...");
            event_tx.send(NetEvent::GossipPublishError {
                correlation_id,
                error: Arc::new(e),
            })?;
        }
    }
    Ok(())
}

fn handle_dial(
    swarm: &mut Swarm<NodeBehaviour>,
    event_tx: &broadcast::Sender<NetEvent>,
    dial_opts: DialOpts,
) -> Result<()> {
    trace!("DIAL: {:?}", dial_opts);
    match swarm.dial(dial_opts) {
        Ok(v) => trace!("Dial returned {:?}", v),
        Err(error) => {
            warn!("Dialing error! {}", error);
            event_tx.send(NetEvent::DialError {
                error: error.into(),
            })?;
        }
    }
    Ok(())
}

fn handle_put_record(
    swarm: &mut Swarm<NodeBehaviour>,
    event_tx: &broadcast::Sender<NetEvent>,
    correlator: &mut Correlator,
    correlation_id: CorrelationId,
    key: Cid,
    expires: Option<Instant>,
    value: ArcBytes,
) -> Result<()> {
    debug!("DHT PUT RECORD");
    let record = Record {
        key: RecordKey::new(&key),
        value: value.extract_bytes(),
        publisher: None, // Will be set automatically to local peer ID
        expires,
    };

    match swarm
        .behaviour_mut()
        .kademlia
        .put_record(record, Quorum::One)
    {
        Ok(query_id) => {
            debug!("PUT RECORD OK {:?}", query_id);
            correlator.track(query_id, correlation_id);
        }
        Err(error) => {
            error!("PUT RECORD ERROR: {:?}", error);
            let err_evt = NetEvent::DhtPutRecordError {
                correlation_id,
                error: error.into(),
            };

            event_tx.send(err_evt)?;
        }
    };
    Ok(())
}

fn handle_get_record(
    swarm: &mut Swarm<NodeBehaviour>,
    correlator: &mut Correlator,
    correlation_id: CorrelationId,
    key: Cid,
) -> Result<()> {
    let query_id = swarm
        .behaviour_mut()
        .kademlia
        .get_record(RecordKey::new(&key));

    // So because of the API above the contract here must
    // be that this will only ever return after some amount of time
    // I could not see a way to specify your own QueryId so we have to
    // track with the correlator
    correlator.track(query_id, correlation_id);

    trace!("get_record sent {:?}", query_id);
    Ok(())
}

fn handle_shutdown(
    swarm: &mut Swarm<NodeBehaviour>,
    shutdown_flag: &Arc<AtomicBool>,
) -> Result<()> {
    info!("Starting graceful shutdown");

    // Set the shutdown flag
    shutdown_flag.store(true, Ordering::Relaxed);

    // Disconnect all peers
    let peers: Vec<_> = swarm.connected_peers().copied().collect();
    for peer in peers {
        info!("Disconnecting from peer: {}", peer);
        let _ = swarm.disconnect_peer_id(peer);
    }

    info!("Graceful shutdown complete");
    Ok(())
}

async fn wait_for_mesh_ready(
    swarm: &mut Swarm<NodeBehaviour>,
    topic: &TopicHash,
    min_peers: usize,
    timeout_duration: Duration,
) -> Result<()> {
    timeout(timeout_duration, async {
        loop {
            let mesh_peers = swarm.behaviour().gossipsub.mesh_peers(topic).count();

            if mesh_peers >= min_peers {
                println!("✓ Mesh ready with {mesh_peers} peers");
                return Ok(());
            }

            println!("Waiting for mesh... ({mesh_peers}/{min_peers} peers)");

            // Process events to allow connections
            tokio::select! {
                _ = swarm.select_next_some() => {},
                _ = sleep(Duration::from_millis(100)) => {},
            }
        }
    })
    .await?
}

#[derive(Debug)]
pub struct MeshParams {
    pub mesh_n: usize,            // D: Sweet spot for redundancy vs overhead
    pub mesh_n_low: usize,        // D_low: Trigger grafting
    pub mesh_n_high: usize,       // D_high: Trigger pruning
    pub mesh_outbound_min: usize, // Min outbound connections
}
impl Default for MeshParams {
    fn default() -> Self {
        Self {
            mesh_n: 6,
            mesh_n_low: 5,
            mesh_n_high: 12,
            mesh_outbound_min: 2,
        }
    }
}
/// Create the libp2p behaviour
fn create_behaviour(
    key: &Keypair,
    mesh_params: MeshParams,
) -> std::result::Result<NodeBehaviour, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let connection_limits = connection_limits::Behaviour::new(ConnectionLimits::default());
    let identify_config = IdentifyBehaviour::new(
        identify::Config::new("/enclave/0.0.1".into(), key.public())
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
        .mesh_n(mesh_params.mesh_n) // D: Sweet spot for redundancy vs overhead
        .mesh_n_low(mesh_params.mesh_n_low) // D_low: Trigger grafting
        .mesh_n_high(mesh_params.mesh_n_high) // D_high: Trigger pruning
        .mesh_outbound_min(mesh_params.mesh_outbound_min) // Min outbound connections
        .message_id_fn(message_id_fn)
        .build()
        .map_err(|msg| Error::new(std::io::ErrorKind::Other, msg))?;

    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(key.clone()),
        gossipsub_config,
    )?;

    let peer_id = key.public().to_peer_id();
    println!("PEER ID = ({})", peer_id);
    let mut kademlia = KademliaBehaviour::new(peer_id, MemoryStore::new(peer_id));

    kademlia.set_mode(Some(kad::Mode::Server));

    Ok(NodeBehaviour {
        gossipsub,
        kademlia,
        connection_limits,
        identify: identify_config,
    })
}

/// Process all swarm events
async fn process_swarm_event(
    swarm: &mut Swarm<NodeBehaviour>,
    event_tx: &broadcast::Sender<NetEvent>,
    correlator: &mut Correlator,
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

        SwarmEvent::Behaviour(NodeBehaviourEvent::Kademlia(
            kad::Event::OutboundQueryProgressed {
                id,
                result: QueryResult::GetRecord(Ok(GetRecordOk::FoundRecord(peer_record))),
                ..
            },
        )) => {
            let key = Cid(peer_record.record.key.to_vec());
            let correlation_id = correlator.expire(&id)?;
            debug!(
                "Received DHT record for key={} correlation_id={}",
                key.to_string(),
                correlation_id
            );
            event_tx.send(NetEvent::DhtGetRecordSucceeded {
                key,
                correlation_id,
                value: ArcBytes::from_bytes(peer_record.record.value),
            })?;
        }

        SwarmEvent::Behaviour(NodeBehaviourEvent::Kademlia(
            kad::Event::OutboundQueryProgressed {
                id,
                result: QueryResult::PutRecord(Ok(record)),
                ..
            },
        )) => {
            let key = Cid(record.key.to_vec());
            let correlation_id = correlator.expire(&id)?;
            debug!(
                "Put DHT record for key={} correlation_id={}",
                key.to_string(),
                correlation_id
            );
            event_tx.send(NetEvent::DhtPutRecordSucceeded {
                key,
                correlation_id,
            })?;
        }

        SwarmEvent::Behaviour(NodeBehaviourEvent::Gossipsub(gossipsub::Event::Message {
            propagation_source: peer_id,
            message_id: id,
            message,
        })) => {
            trace!("Got message with id: {id} from peer: {peer_id}");
            let gossip_data = GossipData::from_bytes(&message.data)?;
            event_tx.send(NetEvent::GossipData(gossip_data))?;
        }

        SwarmEvent::NewListenAddr { address, .. } => {
            trace!("Local node is listening on {address}");
        }

        SwarmEvent::Behaviour(NodeBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed {
            peer_id,
            topic,
        })) => {
            trace!("Peer {} subscribed to {}", peer_id, topic);
            let count = swarm.behaviour().gossipsub.mesh_peers(&topic).count();
            event_tx.send(NetEvent::GossipSubscribed { count, topic })?;
        }

        SwarmEvent::Behaviour(NodeBehaviourEvent::Identify(event)) => {
            if let identify::Event::Received {
                connection_id,
                peer_id,
                info,
            } = event
            {
                for addr in info.listen_addrs {
                    swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                }
            }
        }
        _ => {}
    };
    Ok(())
}

#[derive(Clone)]
struct Correlator {
    inner: HashMap<QueryId, CorrelationId>,
}

impl Correlator {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn track(&mut self, query_id: QueryId, correlation_id: CorrelationId) {
        self.inner.insert(query_id, correlation_id);
    }

    pub fn expire(&mut self, query_id: &QueryId) -> Result<CorrelationId> {
        self.inner
            .remove(query_id)
            .ok_or_else(|| anyhow::anyhow!("Failed to correlate query_id={}", query_id))
    }
}
