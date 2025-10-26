// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{bail, Context, Result};
use e3_events::CorrelationId;
use e3_net::events::{GossipData, NetCommand, NetEvent};
use e3_net::{Cid, MeshParams, NetInterface};
use e3_utils::ArcBytes;
use libp2p::gossipsub::IdentTopic;
use std::time::Duration;
use std::{collections::HashSet, env, process};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info};
use tracing_subscriber::{prelude::*, EnvFilter};

// So this is a simple test to test our networking configuration
// Here we ensure we can send a gossipsub message to all connected nodes
// Each node is assigned a name alice, bob or charlie and expects to receive the other two
// names via gossipsub or the node will exit with an error code
// We have a docker test harness that runs the nodes and blocks things like mdns ports to ensure
// that basic discovery is working

async fn test_gossip(peer: &mut TestPeer) -> Result<()> {
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

    // no need to mark test as finished because all nodes are leading
    Ok(())
}

async fn test_dht(peer: &mut TestPeer) -> Result<()> {
    let value = b"I am he as you are he, as you are me and we are all together";
    let key = Cid::from_content(value);

    if peer.is_lead() {
        // PUT RECORD
        peer.tx
            .send(NetCommand::DhtPutRecord {
                correlation_id: CorrelationId::new(),
                key: key.clone(),
                value: ArcBytes::from_bytes(value.to_vec()),
                expires: None,
            })
            .await?;

        receive_until_collect(
            &mut peer.rx,
            |e| match e {
                NetEvent::DhtPutRecordSucceeded { .. } => true,
                _ => false,
            },
            Duration::from_secs(15),
        )
        .await?;
    }

    peer.sync_nodes().await?;

    // GET RECORD
    peer.tx
        .send(NetCommand::DhtGetRecord {
            correlation_id: CorrelationId::new(),
            key,
        })
        .await?;

    let events = receive_until_collect(
        &mut peer.rx,
        |e| match e {
            NetEvent::DhtGetRecordSucceeded { .. } => true,
            _ => false,
        },
        Duration::from_secs(15),
    )
    .await?;

    let Some(NetEvent::DhtGetRecordSucceeded { value: actual, .. }) = events.last() else {
        return Err(anyhow::anyhow!(
            "Failed to receive success from GET RECORD!"
        ));
    };

    assert_eq!(
        value.to_vec(),
        actual.extract_bytes(),
        "Value does not match!"
    );

    peer.sync_nodes().await?;

    Ok(())
}

async fn receive_until_collect<T, F>(
    rx: &mut broadcast::Receiver<T>,
    predicate: F,
    timeout_duration: Duration,
) -> Result<Vec<T>>
where
    T: Clone,
    F: Fn(&T) -> bool,
{
    let result = timeout(timeout_duration, async {
        let mut results = Vec::new();
        loop {
            let value = rx.recv().await?;
            let matches = predicate(&value);
            results.push(value);
            if matches {
                return Ok::<Vec<T>, broadcast::error::RecvError>(results);
            }
        }
    })
    .await
    .context("Timeout waiting for predicate")?
    .context("Failed to receive from channel")?;

    Ok(result)
}

async fn runner() -> Result<Vec<String>> {
    let mut peer = TestPeer::setup().await?;
    let mut report = vec![];

    // DHT test
    test_dht(&mut peer).await?;
    report.push("DHT Test");

    peer.tx.send(NetCommand::Shutdown).await?;
    sleep(Duration::from_secs(20)).await;

    // Write report
    let report_string = report
        .iter()
        .map(|line| format!("\x1b[32m✓\x1b[0m {}", line))
        .collect::<Vec<String>>();

    Ok(report_string)
}

struct TestPeer {
    name: String,
    sync_threshold: usize,
    rx: broadcast::Receiver<NetEvent>,
    tx: mpsc::Sender<NetCommand>,
    topic: IdentTopic,
    test_timeout: Option<Duration>,
    _running: JoinHandle<()>,
}

static START_SYNC: &[u8] = b"START_SYNC";
static SYNC: &[u8] = b"SYNC";
static END_SYNC: &[u8] = b"END_SYNC";

impl TestPeer {
    // This helps:
    // - ensure nodes are connected and in communication for our tests.
    // - prevents nodes from quitting early before a test has completed if they are not the leader.
    // - tests the gossip pubsub in general
    pub async fn sync_nodes(&mut self) -> Result<()> {
        if self.is_lead() {
            info!("LEAD IS SYNCING");
            self.send_msg(START_SYNC).await?;

            for node in 0..self.sync_threshold {
                debug!(
                    "SYNC: Waiting for reply {}/{}...",
                    node + 1,
                    self.sync_threshold
                );
                self.wait_for_msg(SYNC).await?;
            }

            self.send_msg(END_SYNC).await?;
            info!("LEAD SYNCED!");
        } else {
            info!("FOLLOWER IS SYNCING");
            self.wait_for_msg(START_SYNC).await?;

            self.send_msg(SYNC).await?;

            self.wait_for_msg(END_SYNC).await?;
            info!("FOLLOWER SYNCED!");
        }
        Ok(())
    }

    pub async fn send_msg(&self, msg: &[u8]) -> Result<()> {
        Ok(self
            .tx
            .send(NetCommand::GossipPublish {
                correlation_id: CorrelationId::new(),
                topic: self.topic.to_string(),
                data: GossipData::GossipBytes(msg.to_vec()),
            })
            .await?)
    }

    pub async fn wait_for_msg(&mut self, msg: &[u8]) -> Result<Vec<NetEvent>> {
        Ok(receive_until_collect(
            &mut self.rx,
            |e| match e {
                NetEvent::GossipData(GossipData::GossipBytes(bytes)) => {
                    if msg.to_vec() == bytes.clone() {
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            self.test_timeout.unwrap_or(Duration::from_secs(120)),
        )
        .await?)
    }

    pub fn is_lead(&self) -> bool {
        env::var("TEST_CONFIG").unwrap_or_default().contains("lead")
    }

    async fn setup() -> Result<TestPeer> {
        let name = env::args().nth(2).expect("need name");
        println!("{} starting up", name);

        let udp_port = env::var("QUIC_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok());

        let dial_to = env::var("DIAL_TO")
            .ok()
            .and_then(|p| p.parse::<String>().ok());

        let sync_threshold = env::var("SYNC_THRESHOLD")
            .ok()
            .and_then(|p| p.parse::<usize>().ok())
            .unwrap_or(3);

        let topic = IdentTopic::new("test");

        let peers: Vec<String> = dial_to.iter().cloned().collect();

        let id = libp2p::identity::Keypair::generate_ed25519();
        let mut peer = NetInterface::new(
            &id,
            peers,
            udp_port,
            &topic.to_string(),
            MeshParams {
                mesh_n: 2,
                mesh_n_low: 1,
                mesh_n_high: 3,
                mesh_outbound_min: 1,
            },
        )?;

        // Extract input and outputs
        let tx = peer.tx();
        let mut rx = peer.rx();

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

        println!("WAIT FOR MESH READY...");
        wait_for_mesh_ready(60, 3, &mut rx, &topic).await?;

        println!("MESH READY!");

        // Give network time to initialize
        sleep(Duration::from_secs(3)).await;

        Ok(TestPeer {
            name,
            tx,
            rx,
            sync_threshold,
            topic,
            test_timeout: None,
            _running: router_task,
        })
    }
}

async fn wait_for_mesh_ready(
    seconds: u64,
    min_size: usize,
    rx: &mut broadcast::Receiver<NetEvent>,
    topic: &IdentTopic,
) -> Result<()> {
    let topic_hash = topic.hash();
    loop {
        match timeout(Duration::from_secs(seconds), rx.recv()).await {
            Ok(Ok(NetEvent::GossipSubscribed { count, topic })) => {
                info!(
                    "Received GossipSubscribed with count={}/{} topic={}",
                    count, min_size, topic
                );
                if topic_hash == topic && count >= min_size {
                    break;
                }
            }
            Ok(Err(_)) => break,
            Err(e) => return Err(anyhow::anyhow!("MESH SYNC FAILED!")),
            _ => (),
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    match runner().await {
        Ok(report) => {
            print!("\n\n<<< TEST REPORT >>>\n---------------------------\n{}\n\n---------------------------\n\n",report.join("\n"));
            process::exit(0);
        }
        Err(e) => {
            print!("\n\n<<< FAILURE REPORT >>>\n---------------------------\n{}\n\n---------------------------\n\n",e);
            process::exit(1);
        }
    }
}
