// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_events::CorrelationId;
use e3_utils::ArcBytes;
use libp2p::{
    connection_limits::{self, ConnectionLimits},
    futures::StreamExt,
    gossipsub,
    identify::{self, Behaviour as IdentifyBehaviour},
    identity::Keypair,
    kad::{
        self, store::MemoryStore, Behaviour as KademliaBehaviour, GetRecordOk, QueryId,
        QueryResult, Quorum, Record, RecordKey,
    },
    swarm::{dial_opts::DialOpts, NetworkBehaviour, SwarmEvent},
    Swarm,
};
use std::sync::atomic::AtomicBool;
use std::{
    collections::HashMap,
    sync::{atomic::Ordering, Arc},
    time::Instant,
};
use std::{io::Error, time::Duration};
use tokio::{select, sync::broadcast, sync::mpsc};
use tracing::{debug, error, info, trace, warn};

use crate::events::NetEvent;
use crate::events::{GossipData, NetCommand};
use crate::{dialer::dial_peers, Cid};

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
            .with_behaviour(|key| create_behaviour(key))?
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
                    match process_swarm_command(&mut self.swarm, &event_tx, &shutdown_flag, &mut correlator, command).await {
                        Ok(_) => (),
                        Err(e) => error!("Error processing NetCommand: {e}")
                    }
                }
                // Process events
                event = self.swarm.select_next_some() =>  {
                    match process_swarm_event(&mut self.swarm, &event_tx, &shutdown_flag, &mut correlator, event).await {
                        Ok(_) => (),
                        Err(e) => error!("Error processing NetEvent: {e}")
                    }
                }
            }
        }
    }
}

/// Create the libp2p behaviour
fn create_behaviour(
    key: &Keypair,
) -> std::result::Result<NodeBehaviour, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let peer_id = key.public().to_peer_id();
    let connection_limits = connection_limits::Behaviour::new(ConnectionLimits::default());
    let identify_config = IdentifyBehaviour::new(
        identify::Config::new("/enclave/0.0.1".into(), key.public())
            .with_interval(Duration::from_secs(60)),
    );

    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .build()
        .map_err(|msg| Error::new(std::io::ErrorKind::Other, msg))?;

    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(key.clone()),
        gossipsub_config,
    )?;

    // Setup Kademlia as server so that it responds to events correctly
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
    shutdown_flag: &Arc<AtomicBool>,
    correlator: &mut Correlator,
    event: SwarmEvent<NodeBehaviourEvent>,
) -> Result<()> {
    if shutdown_flag.load(Ordering::Relaxed) {
        return Ok(()); // Skip processing during shutdown
    }
    match event {
        SwarmEvent::ConnectionEstablished {
            peer_id,
            endpoint,
            connection_id,
            ..
        } => {
            info!("Connected to {peer_id}");
            let remote_addr = endpoint.get_remote_address().clone();

            // add address to kademlia
            swarm
                .behaviour_mut()
                .kademlia
                .add_address(&peer_id, remote_addr.clone());

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
                result: QueryResult::GetRecord(result),
                step,
                ..
            },
        )) => match result {
            Ok(GetRecordOk::FoundRecord(record)) => {
                let key = Cid(record.record.key.to_vec());
                let record_bytes = record.record.value;
                let check_key = Cid::from_content(&record_bytes);
                if check_key != key {
                    // Perhaps we do something else here too? maybe this logic should be handled upstream? Not sure...
                    return Err(anyhow::anyhow!(format!(
                        "Received record from peer {:?} but record was invalid ignoring.",
                        record.peer
                    )));
                }
                // As soon as we have a valid record we cancel the query because the record will be large and we can validate the value by hashing the content.
                if let Some(mut query) = swarm.behaviour_mut().kademlia.query_mut(&id) {
                    query.finish();
                }
                let correlation_id = correlator.expire(&id)?;
                debug!(
                    "Received valid DHT record for key={:?} correlation_id={}",
                    key, correlation_id
                );
                event_tx.send(NetEvent::DhtGetRecordSucceeded {
                    key,
                    correlation_id,
                    value: ArcBytes::from_bytes(record_bytes),
                })?;
            }
            Ok(GetRecordOk::FinishedWithNoAdditionalRecord {
                cache_candidates: c,
            }) => {
                trace!("Finished cache={:?} step={:?}", c, step);
            }
            Err(e) => {
                error!("step={:?} error={}", step, e);
                event_tx.send(NetEvent::DhtGetRecordError {
                    correlation_id: correlator.expire(&id)?,
                    error: e,
                })?;
            }
        },

        SwarmEvent::Behaviour(NodeBehaviourEvent::Kademlia(
            kad::Event::OutboundQueryProgressed {
                id,
                result: QueryResult::PutRecord(record),
                ..
            },
        )) => {
            let correlation_id = correlator.expire(&id)?;
            match record {
                Ok(record) => {
                    let key = Cid(record.key.to_vec());
                    info!("PUT RECORD SUCCESS: {:?}", key);
                    event_tx.send(NetEvent::DhtPutRecordSucceeded {
                        key,
                        correlation_id,
                    })?;
                }
                Err(error) => {
                    error!("PUT RECORD FAILED: {}", error);
                    event_tx.send(NetEvent::DhtPutRecordError {
                        correlation_id,
                        error,
                    })?;
                }
            }
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
            info!("Peer {} subscribed to {}", peer_id, topic);
            let count = swarm.behaviour().gossipsub.mesh_peers(&topic).count();
            event_tx.send(NetEvent::GossipSubscribed { count, topic })?;
        }

        _ => {}
    };
    Ok(())
}

/// Process all swarm commands
async fn process_swarm_command(
    swarm: &mut Swarm<NodeBehaviour>,
    event_tx: &broadcast::Sender<NetEvent>,
    shutdown_flag: &Arc<AtomicBool>,
    correlator: &mut Correlator,
    command: NetCommand,
) -> Result<()> {
    if shutdown_flag.load(Ordering::Relaxed) {
        return Ok(()); // Skip processing during shutdown
    }

    match command {
        NetCommand::GossipPublish {
            data,
            topic,
            correlation_id,
        } => handle_gossip_publish(swarm, event_tx, data, topic, correlation_id),
        NetCommand::Dial(multi) => handle_dial(swarm, event_tx, multi),
        NetCommand::DhtPutRecord {
            correlation_id,
            key,
            expires,
            value,
        } => handle_put_record(swarm, correlator, correlation_id, key, expires, value),
        NetCommand::DhtGetRecord {
            correlation_id,
            key,
        } => handle_get_record(swarm, correlator, correlation_id, key),
        NetCommand::Shutdown => handle_shutdown(swarm, shutdown_flag),
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

    let query_id = swarm
        .behaviour_mut()
        .kademlia
        .put_record(record, Quorum::One)?;

    // QueryId is returned synchronously and we immediately add it to the correlator so this should not be an issue.
    correlator.track(query_id, correlation_id);

    info!(
        "PUT RECORD OK query_id={:?} correlation_id={}",
        query_id, correlation_id
    );

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

    // QueryId is returned synchronously and we immediately add it to the correlator so this should not be an issue.
    correlator.track(query_id, correlation_id);

    info!(
        "GET RECORD CORRELATED! query_id={:?} correlation_id={}",
        query_id, correlation_id
    );

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

/// This correlates query_id and correlation_id.
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
    /// Add a pairing between query_id and correlation_id
    pub fn track(&mut self, query_id: QueryId, correlation_id: CorrelationId) {
        self.inner.insert(query_id, correlation_id);
    }
    /// Remove the pairing and return the correlation_id
    pub fn expire(&mut self, query_id: &QueryId) -> Result<CorrelationId> {
        self.inner
            .remove(query_id)
            .ok_or_else(|| anyhow::anyhow!("Failed to correlate query_id={}", query_id))
    }
}
