use alloy::{
    node_bindings::Anvil,
    providers::{ProviderBuilder, WsConnect},
    sol,
};
use enclave_sdk::evm::listener::EventListener;
use eyre::Result;

sol!(
    #[sol(rpc)]
    EmitLogs,
    "tests/fixtures/emit_logs.json"
);

#[tokio::test]
async fn test_event_listener() -> Result<()> {
    let anvil = Anvil::new().block_time(1).try_spawn()?;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);
    let (tx_addr, mut rx_addr) = tokio::sync::mpsc::channel::<String>(10);

    let provider = ProviderBuilder::new()
        .on_ws(WsConnect::new(anvil.ws_endpoint()))
        .await?;

    let contract = EmitLogs::deploy(provider).await?;

    let mut event_listener = EventListener::create_contract_listener(
        &anvil.ws_endpoint(),
        &contract.address().to_string(),
    )
    .await?;

    event_listener
        .add_event_handler(move |event: &EmitLogs::ValueChanged| {
            let _ = tx.clone().try_send(event.value.clone());
            Ok(())
        })
        .await;

    event_listener
        .add_event_handler(move |event: &EmitLogs::ValueChanged| {
            let _ = tx_addr.clone().try_send(event.author.to_string());
            Ok(())
        })
        .await;
    tokio::spawn(async move { event_listener.listen().await.unwrap() });

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
