use actix::{Actor, Addr};
use alloy::{
    node_bindings::Anvil,
    primitives::{FixedBytes, LogData},
    providers::{ProviderBuilder, WsConnect},
    sol,
    sol_types::SolEvent,
};
use anyhow::Result;
use data::Repository;
use enclave_core::{EnclaveEvent, EventBus, GetHistory, Shutdown, TestEvent};
use enclave_node::datastore::get_in_mem_store;
use evm::{helpers::WithChainId, EvmEventReader};
use std::time::Duration;
use tokio::time::sleep;

sol!(
    #[sol(rpc)]
    EmitLogs,
    "tests/fixtures/emit_logs.json"
);

fn test_event_extractor(
    data: &LogData,
    topic: Option<&FixedBytes<32>>,
    _chain_id: u64,
) -> Option<EnclaveEvent> {
    match topic {
        Some(&EmitLogs::ValueChanged::SIGNATURE_HASH) => {
            let Ok(event) = EmitLogs::ValueChanged::decode_log_data(data, true) else {
                return None;
            };
            Some(EnclaveEvent::from(TestEvent {
                msg: event.value,
                entropy: event.count.try_into().unwrap(), // This prevents de-duplication in tests
            }))
        }
        _ => None,
    }
}

#[actix::test]
async fn evm_reader() -> Result<()> {
    // Create a WS provider
    // NOTE: Anvil must be available on $PATH
    let anvil = Anvil::new().block_time(1).try_spawn()?;
    let provider = WithChainId::new(
        ProviderBuilder::new()
            .on_ws(WsConnect::new(anvil.ws_endpoint()))
            .await?,
    )
    .await?;
    let contract = EmitLogs::deploy(provider.get_provider()).await?;
    let bus = EventBus::new(true).start();
    let repository = Repository::new(get_in_mem_store());

    EvmEventReader::attach(
        &provider,
        test_event_extractor,
        &contract.address().to_string(),
        None,
        &bus,
        &repository,
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

#[actix::test]
async fn ensure_historical_events() -> Result<()> {
    // Create a WS provider
    // NOTE: Anvil must be available on $PATH
    let anvil = Anvil::new().block_time(1).try_spawn()?;
    let provider = WithChainId::new(
        ProviderBuilder::new()
            .on_ws(WsConnect::new(anvil.ws_endpoint()))
            .await?,
    )
    .await?;
    let contract = EmitLogs::deploy(provider.get_provider()).await?;
    let bus = EventBus::new(true).start();

    let historical_msgs = vec!["these", "are", "historical", "events"];
    let live_events = vec!["these", "events", "are", "live"];

    let repository = Repository::new(get_in_mem_store());
    for msg in historical_msgs.clone() {
        contract
            .setValue(msg.to_string())
            .send()
            .await?
            .watch()
            .await?;
    }

    EvmEventReader::attach(
        &provider,
        test_event_extractor,
        &contract.address().to_string(),
        None,
        &bus,
        &repository,
    )
    .await?;

    for msg in live_events.clone() {
        contract
            .setValue(msg.to_string())
            .send()
            .await?
            .watch()
            .await?;
    }

    sleep(Duration::from_millis(1)).await;

    let expected: Vec<_> = historical_msgs.into_iter().chain(live_events).collect();

    let history = bus.send(GetHistory).await?;
    assert_eq!(history.len(), 8);

    let msgs: Vec<_> = history
        .into_iter()
        .filter_map(|evt| match evt {
            EnclaveEvent::TestEvent { data, .. } => Some(data.msg),
            _ => None,
        })
        .collect();

    assert_eq!(msgs, expected);

    Ok(())
}

#[actix::test]
async fn ensure_resume_after_shutdown() -> Result<()> {
    // Create a WS provider
    // NOTE: Anvil must be available on $PATH
    let anvil = Anvil::new().block_time(1).try_spawn()?;
    let provider = WithChainId::new(
        ProviderBuilder::new()
            .on_ws(WsConnect::new(anvil.ws_endpoint()))
            .await?,
    )
    .await?;
    let contract = EmitLogs::deploy(provider.get_provider()).await?;
    let bus = EventBus::new(true).start();

    async fn get_msgs(bus: &Addr<EventBus>) -> Result<Vec<String>> {
        let history = bus.send(GetHistory).await?;
        let msgs: Vec<String> = history
            .into_iter()
            .filter_map(|evt| match evt {
                EnclaveEvent::TestEvent { data, .. } => Some(data.msg),
                _ => None,
            })
            .collect();

        Ok(msgs)
    }

    let repository = Repository::new(get_in_mem_store());

    for msg in ["before", "online"] {
        contract
            .setValue(msg.to_string())
            .send()
            .await?
            .watch()
            .await?;
    }

    let addr1 = EvmEventReader::attach(
        &provider,
        test_event_extractor,
        &contract.address().to_string(),
        None,
        &bus,
        &repository,
    )
    .await?;

    for msg in ["live", "events"] {
        contract
            .setValue(msg.to_string())
            .send()
            .await?
            .watch()
            .await?;
    }

    // Ensure shutdown doesn't cause event to be lost.
    sleep(Duration::from_millis(1)).await;
    addr1.send(EnclaveEvent::from(Shutdown)).await?;

    for msg in ["these", "are", "not", "lost"] {
        contract
            .setValue(msg.to_string())
            .send()
            .await?
            .watch()
            .await?;
    }

    sleep(Duration::from_millis(1)).await;
    let msgs = get_msgs(&bus).await?;
    assert_eq!(msgs, ["before", "online", "live", "events"]);

    let _ = EvmEventReader::attach(
        &provider,
        test_event_extractor,
        &contract.address().to_string(),
        None,
        &bus,
        &repository,
    )
    .await?;

    sleep(Duration::from_millis(1)).await;
    let msgs = get_msgs(&bus).await?;
    assert_eq!(
        msgs,
        ["before", "online", "live", "events", "these", "are", "not", "lost"]
    );

    for msg in ["resumed", "data"] {
        contract
            .setValue(msg.to_string())
            .send()
            .await?
            .watch()
            .await?;
    }

    sleep(Duration::from_millis(1)).await;
    let msgs = get_msgs(&bus).await?;
    assert_eq!(
        msgs,
        ["before", "online", "live", "events", "these", "are", "not", "lost", "resumed", "data"]
    );

    Ok(())
}
