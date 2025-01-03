use actix::prelude::*;
use anyhow::Result;
use events::{EventBus, EventBusConfig, GetHistory};
use libp2p::gossipsub;
use net::correlation_id::CorrelationId;
use net::events::{NetworkPeerCommand, NetworkPeerEvent};
use net::DialerActor;
use net::NetworkPeer;
use std::time::Duration;
use std::{collections::HashSet, env, process};
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};
use tracing_subscriber::{prelude::*, EnvFilter};

// So this is a simple test to test our networking configuration
// Here we ensure we can send a gossipsub message to all connected nodes
// Each node is assigned a name alice, bob or charlie and expects to receive the other two
// names via gossipsub or the node will exit with an error code
// We have a docker test harness that runs the nodes and blocks things like mdns ports to ensure
// that basic discovery is working

#[actix::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();
    let name = env::args().nth(2).expect("need name");
    let topic = "test-topic";
    println!("{} starting up", name);

    let udp_port = env::var("QUIC_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok());

    let dial_to = env::var("DIAL_TO")
        .ok()
        .and_then(|p| p.parse::<String>().ok());

    let enable_mdns = env::var("ENABLE_MDNS")
        .unwrap_or("false".to_string())
        .parse::<bool>()
        .unwrap();

    let peers: Vec<String> = dial_to.iter().cloned().collect();

    let id = libp2p::identity::Keypair::generate_ed25519();
    let (tx, rx) = mpsc::channel(100);

    let net_bus = EventBus::<NetworkPeerEvent>::new(EventBusConfig {
        capture_history: true,
        deduplicate: false,
    })
    .start();

    let mut peer = NetworkPeer::new(&id, enable_mdns, net_bus.clone(), rx)?;
    let topic_id = gossipsub::IdentTopic::new(topic);
    peer.subscribe(&topic_id)?;
    peer.listen_on(udp_port.unwrap_or(0))?;

    let name_clone = name.clone();
    let swarm_handle = tokio::spawn(async move {
        println!("{} starting swarm", name_clone);
        if let Err(e) = peer.start().await {
            println!("{} swarm failed: {}", name_clone, e);
        }
        println!("{} swarm finished", name_clone);
    });

    // Give network time to initialize
    sleep(Duration::from_secs(3)).await;

    // Set up dialer for peers
    for peer in peers {
        DialerActor::dial_peer(peer, net_bus.clone(), tx.clone());
    }

    // Send our message first
    println!("{} sending message", name);
    tx.send(NetworkPeerCommand::GossipPublish {
        correlation_id: CorrelationId::new(),
        topic: topic.to_string(),
        data: name.as_bytes().to_vec(),
    })
    .await?;
    println!("{} message sent", name);

    let expected: HashSet<String> = vec![
        "alice".to_string(),
        "bob".to_string(),
        "charlie".to_string(),
    ]
    .into_iter()
    .filter(|n| *n != name)
    .collect();
    println!("{} waiting for messages from: {:?}", name, expected);

    // Then wait to receive from others with a timeout
    let mut received = HashSet::new();
    let receive_result = timeout(Duration::from_secs(10), async {
        let history = net_bus.send(GetHistory::<NetworkPeerEvent>::new()).await?;
        println!("{} history: {:?}", name, history);
        while received != expected {
            for event in history.clone() {
                match event {
                    NetworkPeerEvent::GossipData(msg) => {
                        println!(
                            "{} received '{}'",
                            name,
                            String::from_utf8(msg.clone()).unwrap()
                        );
                        received.insert(String::from_utf8(msg).unwrap());
                    }
                    _ => (),
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    })
    .await;

    match receive_result {
        Ok(Ok(())) => {
            println!("{} received all expected messages", name);
        }
        Ok(Err(e)) => {
            println!("{} error while receiving messages: {}", name, e);
            process::exit(1);
        }
        Err(_) => {
            println!(
                "{} timeout waiting for messages. Received only: {:?}",
                name, received
            );
            process::exit(1);
        }
    }

    // Make sure router task is still running
    if swarm_handle.is_finished() {
        println!("{} warning: swarm task finished early", name);
    }

    // Give some time for final message propagation
    sleep(Duration::from_secs(1)).await;
    println!("{} finished successfully", name);
    Ok(())
}
