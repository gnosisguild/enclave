mod helpers;
use alloy::sol;
use e3_evm_helpers::listener::EventListener;
use eyre::Result;
use helpers::setup_logs_contract;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

sol!(
    #[sol(rpc)]
    EmitLogs,
    "tests/fixtures/emit_logs.json"
);

#[tokio::test]
async fn test_event_listener() -> Result<()> {
    let (contract, _, _, anvil) = setup_logs_contract().await?;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);
    let (tx_addr, mut rx_addr) = tokio::sync::mpsc::channel::<String>(10);

    let mut event_listener = EventListener::create_contract_listener(
        &anvil.ws_endpoint(),
        &contract.address().to_string(),
    )
    .await?;

    event_listener
        .add_event_handler(move |event: EmitLogs::ValueChanged| {
            let tx = tx.clone();
            async move {
                let _ = tx.try_send(event.value.clone());
                Ok(())
            }
        })
        .await;

    event_listener
        .add_event_handler(move |event: EmitLogs::ValueChanged| {
            let tx_addr = tx_addr.clone();
            async move {
                let _ = tx_addr.try_send(event.author.to_string());
                Ok(())
            }
        })
        .await;

    event_listener.start();

    contract
        .setValue("hello".to_string())
        .send()
        .await?
        .watch()
        .await?;

    contract
        .setValue("world!".to_string())
        .send()
        .await?
        .watch()
        .await?;

    assert_eq!(rx.recv().await.unwrap(), "hello");
    assert_eq!(rx.recv().await.unwrap(), "world!");

    assert_eq!(
        rx_addr.recv().await.unwrap(),
        "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
    );
    assert_eq!(
        rx_addr.recv().await.unwrap(),
        "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
    );
    Ok(())
}

fn time_diff(past_timestamp: u128) -> Result<String> {
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let time_diff = current_time.saturating_sub(past_timestamp);
    Ok(format!("{}ms", time_diff))
}

fn process_message_with_timestamp(input: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = input.splitn(2, ':').collect();
    let message = parts[0].to_string();
    let timestamp_str = parts[1].trim();
    let past_timestamp: u128 = timestamp_str.parse()?;
    let time_diff_string = time_diff(past_timestamp)?;
    Ok((message, time_diff_string))
}

#[tokio::test]
async fn test_overlapping_listener_handlers() -> Result<()> {
    // Test that listeners can have overlapping async handlers.
    // Long running handlers should run async while other handlers respond to
    // events without disruption.
    let (contract, _, _, anvil) = setup_logs_contract().await?;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);

    let mut event_listener = EventListener::create_contract_listener(
        &anvil.ws_endpoint(),
        &contract.address().to_string(),
    )
    .await?;

    let tx1 = tx.clone();
    event_listener
        .add_event_handler(move |event: EmitLogs::PublishMessage| {
            let tx = tx1.clone();
            async move {
                let (msg, time_diff) = process_message_with_timestamp(&event.value)?;
                println!("PublishMessage '{}' ({} since sent)", msg, time_diff);

                let _ = tx.try_send("waiting".to_string());
                // Wait 200ms before publishing to simulate long running handlers
                sleep(Duration::from_millis(200)).await;
                println!("Sending message: '{msg}'");
                let _ = tx.try_send(msg);
                Ok(())
            }
        })
        .await;

    event_listener
        .add_event_handler(move |event: EmitLogs::ValueChanged| {
            let tx = tx.clone();
            async move {
                let (msg, time_diff) = process_message_with_timestamp(&event.value)?;
                println!("ValueChanged '{}' ({} since sent)", msg, time_diff);
                let _ = tx.try_send(msg);
                Ok(())
            }
        })
        .await;

    event_listener.start();

    // Events should be returned roughly in this order:
    // 0ms : one
    // 0ms : waiting
    // 100ms : two
    // 200ms : three
    // 300ms : four

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    contract
        .setValue(format!("one:{now}"))
        .send()
        .await?
        .watch()
        .await?;

    // Will delay 200ms
    contract
        .emitPublishMessage(format!("three:{now}"))
        .send()
        .await?
        .watch()
        .await?;

    sleep(Duration::from_millis(100)).await;

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    contract
        .setValue(format!("two:{now}"))
        .send()
        .await?
        .watch()
        .await?;

    sleep(Duration::from_millis(300)).await;

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    contract
        .setValue(format!("four:{now}"))
        .send()
        .await?
        .watch()
        .await?;

    assert_eq!(rx.recv().await.unwrap(), "one");
    assert_eq!(rx.recv().await.unwrap(), "waiting");
    assert_eq!(rx.recv().await.unwrap(), "two");
    assert_eq!(rx.recv().await.unwrap(), "three");
    assert_eq!(rx.recv().await.unwrap(), "four");

    Ok(())
}
