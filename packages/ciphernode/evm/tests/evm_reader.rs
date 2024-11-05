use actix::Actor;
use alloy::{
    node_bindings::Anvil,
    providers::{ProviderBuilder, WsConnect},
    sol,
    sol_types::SolEvent,
};
use anyhow::Result;
use enclave_core::{EnclaveEvent, EventBus, GetHistory, TestEvent};
use evm::{helpers::WithChainId, EvmEventReader};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

sol!(
    #[sol(rpc)]
    EmitLogs,
    "tests/fixtures/emit_logs.json"
);

#[actix::test]
async fn test_logs() -> Result<()> {
    // Create a WS provider
    // NOTE: Anvil must be available on $PATH
    let anvil = Anvil::new().block_time(1).try_spawn()?;
    let ws = WsConnect::new(anvil.ws_endpoint());
    let provider = ProviderBuilder::new().on_ws(ws).await?;
    let arc_provider = WithChainId::new(provider).await?;
    let contract = Arc::new(EmitLogs::deploy(arc_provider.get_provider()).await?);
    let bus = EventBus::new(true).start();

    EvmEventReader::attach(
        &bus,
        &arc_provider,
        |data, topic, _| match topic {
            Some(&EmitLogs::ValueChanged::SIGNATURE_HASH) => {
                let Ok(event) = EmitLogs::ValueChanged::decode_log_data(data, true) else {
                    return None;
                };
                Some(EnclaveEvent::from(TestEvent {
                    msg: event.newValue,
                }))
            }
            _ => None,
        },
        &contract.address().to_string(),
    )
    .await?;

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

    sleep(Duration::from_millis(1)).await;

    let history = bus.send(GetHistory).await?;

    assert_eq!(history.len(), 2);

    let msgs: Vec<_> = history
        .into_iter()
        .filter_map(|evt| match evt {
            EnclaveEvent::TestEvent { data, .. } => Some(data.msg),
            _ => None,
        })
        .collect();

    assert_eq!(msgs, vec!["hello", "world!"]);

    Ok(())
}
