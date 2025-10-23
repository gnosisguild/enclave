// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{bail, Result};
use e3_events::CorrelationId;
use e3_net::events::{GossipData, NetCommand, NetEvent};
use e3_net::{Cid, NetInterface};
use e3_utils::ArcBytes;
use std::time::Duration;
use std::{collections::HashSet, env, process};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tracing_subscriber::{prelude::*, EnvFilter};

// So this is a simple test to test our networking configuration
// Here we ensure we can send a gossipsub message to all connected nodes
// Each node is assigned a name alice, bob or charlie and expects to receive the other two
// names via gossipsub or the node will exit with an error code
// We have a docker test harness that runs the nodes and blocks things like mdns ports to ensure
// that basic discovery is working

async fn test_gossip(peer: &mut PeerHandle) -> Result<()> {
    let topic = "test-topic";
    let name = peer.name.clone();
    println!("{} starting up", name);

    // Send our message first
    println!("{} sending message", name);
    peer.tx
        .send(NetCommand::GossipPublish {
            correlation_id: CorrelationId::new(),
            topic: topic.to_string(),
            data: GossipData::GossipBytes(name.as_bytes().to_vec()),
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

    // Wrap the message receiving loop in a timeout
    let receive_result = timeout(Duration::from_secs(10), async {
        while received != expected {
            match peer.rx.recv().await? {
                NetEvent::GossipData(GossipData::GossipBytes(msg)) => {
                    match String::from_utf8(msg) {
                        Ok(msg) => {
                            if !received.contains(&msg) {
                                println!("{} received '{}'", name, msg);
                                received.insert(msg);
                            }
                        }
                        Err(e) => println!("{} received invalid UTF8: {}", name, e),
                    }
                }
                _ => (),
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
            bail!("{} error while receiving messages: {}", name, e);
        }
        Err(_) => {
            bail!(
                "{} timeout waiting for messages. Received only: {:?}",
                name,
                received
            );
        }
    }

    // Make sure router task is still running
    if peer.running.is_finished() {
        println!("{} warning: router task finished early", name);
    }

    // Give some time for final message propagation
    sleep(Duration::from_secs(1)).await;
    println!("{} finished successfully", name);
    Ok(())
}

async fn test_dht(peer: &mut PeerHandle) -> Result<()> {
    let value = b"I am he as you are he, as you are me and we are all together";
    peer.tx
        .send(NetCommand::DhtPutRecord {
            correlation_id: CorrelationId::new(),
            key: Cid::from_content(value),
            value: ArcBytes::from_bytes(value.to_vec()),
            expires: None,
        })
        .await?;

    let NetEvent::DhtPutRecordSucceeded { correlation_id, .. } =
        timeout(Duration::from_secs(4), peer.rx.recv()).await??
    else {
        bail!("msg not as expected");
    };

    Ok(())
}

async fn runner() -> Result<()> {
    let mut peer = setup_peer().await?;
    test_gossip(&mut peer).await?;
    test_dht(&mut peer).await?;
    Ok(())
}

struct PeerHandle {
    name: String,
    rx: broadcast::Receiver<NetEvent>,
    tx: mpsc::Sender<NetCommand>,
    running: JoinHandle<()>,
}

async fn setup_peer() -> Result<PeerHandle> {
    let name = env::args().nth(2).expect("need name");
    println!("{} starting up", name);

    let udp_port = env::var("QUIC_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok());

    let dial_to = env::var("DIAL_TO")
        .ok()
        .and_then(|p| p.parse::<String>().ok());

    let peers: Vec<String> = dial_to.iter().cloned().collect();

    let id = libp2p::identity::Keypair::generate_ed25519();
    let mut peer = NetInterface::new(&id, peers, udp_port, "test-topic")?;

    // Extract input and outputs
    let tx = peer.tx();
    let rx = peer.rx();

    let router_task = tokio::spawn({
        let name = name.clone();
        async move {
            println!("{} starting router task", name);
            if let Err(e) = peer.start().await {
                println!("{} router task failed: {}", name, e);
            }
            println!("{} router task finished", name);
        }
    });

    // Give network time to initialize
    sleep(Duration::from_secs(3)).await;
    Ok(PeerHandle {
        name,
        tx,
        rx,
        running: router_task,
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    match runner().await {
        Ok(()) => {
            println!("SUCCESS!");
            process::exit(0);
        }
        Err(e) => {
            eprintln!("FAILURE: {e}");
            process::exit(1);
        }
    }
}
