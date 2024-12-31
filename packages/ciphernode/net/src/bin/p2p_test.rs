use actix::prelude::*;
use anyhow::Result;
use libp2p::identity::Keypair;
use std::{collections::HashSet, env, process, time::Instant};
use std::time::Duration;
use tracing::{error, info};
use tracing_subscriber::{prelude::*, EnvFilter};

use net::correlation_id::CorrelationId;
use net::events::{NetworkPeerCommand, NetworkPeerEvent};
use net::{ NetworkPeer, SetNetworkManager, SubscribeTopic};

struct TestManager {
    name: String,
    expected: HashSet<String>,
    received: HashSet<String>,
    start_time: Instant,
    timeout_secs: u64,
}

impl TestManager {
    fn new(name: &str, expected: HashSet<String>, timeout_secs: u64) -> Self {
        Self {
            name: name.to_string(),
            expected,
            received: HashSet::new(),
            start_time: Instant::now(),
            timeout_secs,
        }
    }
}

impl Actor for TestManager {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("TestManager for '{}' started", self.name);

        ctx.run_interval(Duration::from_millis(100), |act, _ctx| {
            if act.received == act.expected {
                info!(
                    "{} received all expected messages: {:?}",
                    act.name, act.received
                );
                System::current().stop();
                return;
            }

            if Instant::now().duration_since(act.start_time).as_secs() > act.timeout_secs {
                error!(
                    "{} timed out. Received only {:?}, but still expected {:?}",
                    act.name, act.received, act.expected
                );
                process::exit(1);
            }
        });
    }
}

impl Handler<NetworkPeerEvent> for TestManager {
    type Result = ();

    fn handle(&mut self, event: NetworkPeerEvent, _ctx: &mut Self::Context) -> Self::Result {
        match event {
            NetworkPeerEvent::GossipData(bytes) => {
                info!("{} received data", self.name);
                match String::from_utf8(bytes) {
                    Ok(peer_name) => {
                        if !self.received.contains(&peer_name) {
                            info!("{} received '{}'", self.name, peer_name);
                            self.received.insert(peer_name);
                        }
                    }
                    Err(e) => {
                        error!("{} received invalid UTF8: {}", self.name, e);
                    }
                }
            }
            NetworkPeerEvent::GossipPublished { correlation_id, message_id } => {
                info!(
                    "{} successfully published message with ID {:?} and correlation ID {:?}",
                    self.name, message_id, correlation_id
                );
            }
            NetworkPeerEvent::GossipPublishError { error, .. } => {
                error!("{} received GossipPublishError: {:?}", self.name, error);
                process::exit(1);
            }
            NetworkPeerEvent::ConnectionEstablished { connection_id } => {
                info!("{}: connection established (id={})", self.name, connection_id);
            }
            NetworkPeerEvent::DialError { error, connection_id } => {
                info!(
                    "{}: dial error on connection {}: {}",
                    self.name, connection_id, error
                );
            }
            NetworkPeerEvent::OutgoingConnectionError {
                connection_id,
                error,
                ..
            } => {
                error!(
                    "{}: outgoing connection error on {}: {}",
                    self.name, connection_id, error
                );
            }
        }
    }
}


#[actix::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let name = env::args().nth(2).expect("need name argument");
    info!("{} starting up", name);

    // Same environment variables as your old test
    let udp_port = env::var("QUIC_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok());
    let dial_to = env::var("DIAL_TO").ok();
    let enable_mdns = env::var("ENABLE_MDNS")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()?;

    let mut peers = vec![];
    if let Some(dial_str) = dial_to {
        peers.push(dial_str);
    }

    let topic = "test-topic";
    let id = Keypair::generate_ed25519();
    let peer_addr = NetworkPeer::setup(&id, peers.clone(), udp_port, enable_mdns);

    let mut all_nodes = vec!["alice", "bob", "charlie"];
    all_nodes.retain(|n| *n != name);
    let expected: HashSet<String> = all_nodes.iter().map(|s| s.to_string()).collect();
    let test_manager = TestManager::new(&name, expected, 10).start();

    peer_addr
        .send(SetNetworkManager(test_manager.recipient()))
        .await??;

    peer_addr
        .send(SubscribeTopic(topic.to_string()))
        .await??;

    peer_addr.do_send(NetworkPeerCommand::GossipPublish {
        correlation_id: CorrelationId::new(),
        topic: topic.to_string(),
        data: name.as_bytes().to_vec(),
    });

    Ok(())
}
