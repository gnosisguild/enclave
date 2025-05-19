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
async fn listener() -> Result<()> {
    let anvil = Anvil::new().block_time(1).try_spawn()?;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);

    let provider = ProviderBuilder::new()
        .on_ws(WsConnect::new(anvil.ws_endpoint()))
        .await?;

    let contract = EmitLogs::deploy(provider).await?;

    let mut dispatcher =
        EventListener::create_contract_listener(&anvil.ws_endpoint(), contract.address()).await?;

    dispatcher.add_event_handler::<EmitLogs::ValueChanged>(
        move |event: &EmitLogs::ValueChanged| {
            let _ = tx.clone().try_send(event.value.clone());
            Ok(())
        },
    );

    tokio::spawn(async move { dispatcher.listen().await.unwrap() });

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

    Ok(())
}
