use anyhow::Result;
use p2p::EnclaveRouter;
use std::time::Duration;
use std::{collections::HashSet, env};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    let name = env::args().nth(1).expect("need name");
    println!("{} starting up", name);

    let (mut router, tx, mut rx) = EnclaveRouter::new()?;
    let keypair = libp2p::identity::Keypair::generate_ed25519();
    router
        .with_identity(&keypair)
        .connect_swarm()?
        .join_topic("test-topic")?;

    let router_task = tokio::spawn({
        let name = name.clone();
        async move {
            println!("{} starting router task", name);
            if let Err(e) = router.start().await {
                println!("{} router task failed: {}", name, e);
            }
            println!("{} router task finished", name);
        }
    });

    // Give network time to initialize
    sleep(Duration::from_secs(1)).await;

    // Send our message first
    println!("{} sending message", name);
    tx.send(name.as_bytes().to_vec()).await?;
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

    // Then wait to receive from others
    let mut received = HashSet::new();
    while received != expected {
        if let Some(msg) = rx.recv().await {
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
    }

    println!("{} received all expected messages", name);

    // Make sure router task is still running
    if router_task.is_finished() {
        println!("{} warning: router task finished early", name);
    }

    // Give some time for final message propagation
    sleep(Duration::from_secs(1)).await;

    println!("{} finished successfully", name);
    Ok(())
}
