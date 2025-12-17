// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Addr;
use alloy::{
    node_bindings::Anvil,
    primitives::{FixedBytes, LogData},
    providers::{ProviderBuilder, WsConnect},
    signers::local::PrivateKeySigner,
    sol,
    sol_types::SolEvent,
};
use anyhow::Result;
use e3_ciphernode_builder::EventSystem;
use e3_data::Repository;
use e3_entrypoint::helpers::datastore::get_in_mem_store;
use e3_events::{
    prelude::*, EnclaveEvent, EnclaveEventData, GetEvents, HistoryCollector, Shutdown, TestEvent,
};
use e3_evm::{helpers::EthProvider, CoordinatorStart, EvmEventReader, HistoricalEventCoordinator};
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
) -> Option<EnclaveEventData> {
    match topic {
        Some(&EmitLogs::ValueChanged::SIGNATURE_HASH) => {
            let Ok(event) = EmitLogs::ValueChanged::decode_log_data(data) else {
                return None;
            };
            Some(
                TestEvent {
                    msg: event.value,
                    entropy: event.count.try_into().unwrap(), // This prevents de-duplication in tests
                }
                .into(),
            )
        }
        _ => None,
    }
}

async fn get_msgs(history_collector: &Addr<HistoryCollector<EnclaveEvent>>) -> Result<Vec<String>> {
    let history = history_collector
        .send(GetEvents::<EnclaveEvent>::new())
        .await?;
    let msgs: Vec<String> = history
        .into_iter()
        .filter_map(|evt| match evt.into_data() {
            EnclaveEventData::TestEvent(data) => Some(data.msg),
            _ => None,
        })
        .collect();

    Ok(msgs)
}

#[actix::test]
async fn evm_reader() -> Result<()> {
    // Create a WS provider
    // NOTE: Anvil must be available on $PATH
    let anvil = Anvil::new().block_time(1).try_spawn()?;
    let rpc_url = anvil.ws_endpoint(); // Get RPC URL
    let provider = EthProvider::new(
        ProviderBuilder::new()
            .wallet(PrivateKeySigner::from_slice(&anvil.keys()[0].to_bytes())?)
            .connect_ws(WsConnect::new(rpc_url.clone())) // Use RPC URL
            .await?,
    )
    .await?;
    let contract = EmitLogs::deploy(provider.provider()).await?;
    let system = EventSystem::new("test").with_fresh_bus();
    let bus = system.handle()?;
    let history_collector = bus.history();
    let repository = Repository::new(get_in_mem_store());

    let coordinator = HistoricalEventCoordinator::setup(bus.clone());
    let processor = coordinator.clone().recipient();

    EvmEventReader::attach(
        provider.clone(),
        test_event_extractor,
        &contract.address().to_string(),
        None,
        &processor,
        &bus,
        &repository,
        rpc_url.clone(),
    )
    .await?;

    coordinator.do_send(CoordinatorStart);

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

    let history = history_collector
        .send(GetEvents::<EnclaveEvent>::new())
        .await?;

    assert_eq!(history.len(), 2);

    let msgs: Vec<_> = history
        .into_iter()
        .filter_map(|evt| match evt.into_data() {
            EnclaveEventData::TestEvent(data) => Some(data.msg),
            _ => None,
        })
        .collect();

    assert_eq!(msgs, vec!["hello", "world!"]);

    Ok(())
}

/// Verifies that events emitted before an EVM reader is attached (historical events)
/// are captured and returned in order together with subsequently emitted live events.
///
/// The test starts an Anvil WebSocket provider, deploys the EmitLogs contract,
/// emits a sequence of historical events, attaches an EvmEventReader, emits live events,
/// and asserts that the history contains the historical events followed by the live events.
///
/// # Examples
///
/// ```
/// // This test is executed by the test harness (cargo test). To exercise the same
/// // behavior manually from an async context:
/// # async fn run() -> eyre::Result<()> {
/// #     // call the test function as an async function in examples only; normally
/// #     // the test framework runs it automatically.
/// #     let _ = crate::ensure_historical_events().await?;
/// #     Ok(())
/// # }
/// ```
#[actix::test]
async fn ensure_historical_events() -> Result<()> {
    // Create a WS provider
    // NOTE: Anvil must be available on $PATH
    let anvil = Anvil::new().block_time(1).try_spawn()?;
    let rpc_url = anvil.ws_endpoint(); // Get RPC URL
    let provider = EthProvider::new(
        ProviderBuilder::new()
            .wallet(PrivateKeySigner::from_slice(&anvil.keys()[0].to_bytes())?)
            .connect_ws(WsConnect::new(rpc_url.clone())) // Use RPC URL
            .await?,
    )
    .await?;
    let contract = EmitLogs::deploy(provider.provider()).await?;
    let system = EventSystem::new("test").with_fresh_bus();
    let bus = system.handle()?;
    let history_collector = bus.history();
    let historical_msgs = vec!["these", "are", "historical", "events"];
    let live_events = vec!["these", "events", "are", "live"];

    let repository = Repository::new(get_in_mem_store());

    let coordinator = HistoricalEventCoordinator::setup(bus.clone());
    let processor = coordinator.clone().recipient();

    for msg in historical_msgs.clone() {
        contract
            .setValue(msg.to_string())
            .send()
            .await?
            .watch()
            .await?;
    }

    EvmEventReader::attach(
        provider.clone(),
        test_event_extractor,
        &contract.address().to_string(),
        None,
        &processor,
        &bus,
        &repository,
        rpc_url.clone(),
    )
    .await?;

    coordinator.do_send(CoordinatorStart);

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

    let history = history_collector
        .send(GetEvents::<EnclaveEvent>::new())
        .await?;

    assert_eq!(history.len(), 8);

    let msgs: Vec<_> = history
        .into_iter()
        .filter_map(|evt| match evt.into_data() {
            EnclaveEventData::TestEvent(data) => Some(data.msg),
            _ => None,
        })
        .collect();

    assert_eq!(msgs, expected);

    Ok(())
}

/// Verifies that an EVM event reader resumes after a shutdown and that no events are lost during the shutdown window.
///
/// The test:
/// - Emits events before any reader is attached (historical events).
/// - Attaches an `EvmEventReader` and starts the coordinator so live events are processed.
/// - Emits additional live events, then simulates a reader shutdown by storing a `Shutdown` enclave event.
/// - Emits further events while the reader is down, re-attaches a reader, and asserts that all events
///   (pre-shutdown historical, live before shutdown, events emitted during shutdown, and events after resume)
///   appear in the collected history in the original emission order.
///
/// # Examples
///
/// ```no_run
/// # use eyre::Result;
/// # async fn example() -> Result<()> {
/// ensure_resume_after_shutdown().await?;
/// # Ok(()) }
/// ```
#[actix::test]
async fn ensure_resume_after_shutdown() -> Result<()> {
    // Create a WS provider
    // NOTE: Anvil must be available on $PATH
    let anvil = Anvil::new().block_time(1).try_spawn()?;
    let rpc_url = anvil.ws_endpoint(); // Get RPC URL
    let provider = EthProvider::new(
        ProviderBuilder::new()
            .wallet(PrivateKeySigner::from_slice(&anvil.keys()[0].to_bytes())?)
            .connect_ws(WsConnect::new(rpc_url.clone())) // Use RPC URL
            .await?,
    )
    .await?;
    let contract = EmitLogs::deploy(provider.provider()).await?;
    let system = EventSystem::new("test").with_fresh_bus();
    let bus = system.handle()?;
    let history_collector = bus.history();
    let repository = Repository::new(get_in_mem_store());

    let coordinator = HistoricalEventCoordinator::setup(bus.clone());
    let processor = coordinator.clone().recipient();

    for msg in ["before", "online"] {
        contract
            .setValue(msg.to_string())
            .send()
            .await?
            .watch()
            .await?;
    }

    let addr1 = EvmEventReader::attach(
        provider.clone(),
        test_event_extractor,
        &contract.address().to_string(),
        None,
        &processor,
        &bus,
        &repository,
        rpc_url.clone(),
    )
    .await?;

    coordinator.do_send(CoordinatorStart);

    for msg in ["live", "events"] {
        contract
            .setValue(msg.to_string())
            .send()
            .await?
            .watch()
            .await?;
    }

    // Ensure shutdown doesn't cause event to be lost.
    sleep(Duration::from_millis(10)).await;
    addr1
        .send(EnclaveEvent::new_stored_event(Shutdown.into(), 4321, 42))
        .await?;

    for msg in ["these", "are", "not", "lost"] {
        contract
            .setValue(msg.to_string())
            .send()
            .await?
            .watch()
            .await?;
    }

    sleep(Duration::from_millis(10)).await;
    let msgs = get_msgs(&history_collector).await?;
    assert_eq!(msgs, ["before", "online", "live", "events"]);

    let _ = EvmEventReader::attach(
        provider.clone(),
        test_event_extractor,
        &contract.address().to_string(),
        None,
        &processor,
        &bus,
        &repository,
        rpc_url.clone(),
    )
    .await?;

    sleep(Duration::from_millis(10)).await;
    let msgs = get_msgs(&history_collector).await?;
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

    sleep(Duration::from_millis(10)).await;
    let msgs = get_msgs(&history_collector).await?;
    assert_eq!(
        msgs,
        ["before", "online", "live", "events", "these", "are", "not", "lost", "resumed", "data"]
    );

    Ok(())
}

/// Integration test that verifies a single EVM reader processes both historical events and subsequent live events in order.
///
/// This test deploys an EmitLogs contract to an Anvil node, stages several historical events by emitting logs
/// before attaching the reader, attaches an EvmEventReader via a HistoricalEventCoordinator, starts the
/// coordinator, and then emits live events to ensure they are processed after the historical ones.
///
/// # Examples
///
/// ```
/// # async fn run() -> anyhow::Result<()> {
/// coordinator_single_reader().await?;
/// # Ok::<(), anyhow::Error>(())
/// # }
/// ```
#[actix::test]
async fn coordinator_single_reader() -> Result<()> {
    let anvil = Anvil::new().block_time(1).try_spawn()?;
    let rpc_url = anvil.ws_endpoint();
    let provider = EthProvider::new(
        ProviderBuilder::new()
            .wallet(PrivateKeySigner::from_slice(&anvil.keys()[0].to_bytes())?)
            .connect_ws(WsConnect::new(rpc_url.clone()))
            .await?,
    )
    .await?;
    let contract = EmitLogs::deploy(provider.provider()).await?;
    let system = EventSystem::new("test").with_fresh_bus();
    let bus = system.handle()?;
    let history_collector = bus.history();
    let repository = Repository::new(get_in_mem_store());

    let coordinator = HistoricalEventCoordinator::setup(bus.clone());
    let processor = coordinator.clone().recipient();

    for msg in ["historical1", "historical2", "historical3"] {
        contract
            .setValue(msg.to_string())
            .send()
            .await?
            .watch()
            .await?;
    }

    EvmEventReader::attach(
        provider.clone(),
        test_event_extractor,
        &contract.address().to_string(),
        None,
        &processor,
        &bus,
        &repository,
        rpc_url.clone(),
    )
    .await?;

    coordinator.do_send(CoordinatorStart);
    sleep(Duration::from_millis(100)).await;

    let msgs = get_msgs(&history_collector).await?;
    assert_eq!(msgs, ["historical1", "historical2", "historical3"]);

    for msg in ["live1", "live2"] {
        contract
            .setValue(msg.to_string())
            .send()
            .await?
            .watch()
            .await?;
    }

    sleep(Duration::from_millis(100)).await;
    let msgs = get_msgs(&history_collector).await?;
    assert_eq!(
        msgs,
        [
            "historical1",
            "historical2",
            "historical3",
            "live1",
            "live2"
        ]
    );

    Ok(())
}

#[actix::test]
async fn coordinator_multiple_readers() -> Result<()> {
    let anvil = Anvil::new().block_time(1).try_spawn()?;
    let rpc_url = anvil.ws_endpoint();
    let provider = EthProvider::new(
        ProviderBuilder::new()
            .wallet(PrivateKeySigner::from_slice(&anvil.keys()[0].to_bytes())?)
            .connect_ws(WsConnect::new(rpc_url.clone()))
            .await?,
    )
    .await?;

    let contract1 = EmitLogs::deploy(provider.provider()).await?;
    let contract2 = EmitLogs::deploy(provider.provider()).await?;

    let system = EventSystem::new("test").with_fresh_bus();
    let bus = system.handle()?;
    let history_collector = bus.history();
    let repository1 = Repository::new(get_in_mem_store());
    let repository2 = Repository::new(get_in_mem_store());

    let coordinator = HistoricalEventCoordinator::setup(bus.clone());
    let processor = coordinator.clone().recipient();

    contract1
        .setValue("contract1_msg1".to_string())
        .send()
        .await?
        .watch()
        .await?;
    contract2
        .setValue("contract2_msg1".to_string())
        .send()
        .await?
        .watch()
        .await?;
    contract1
        .setValue("contract1_msg2".to_string())
        .send()
        .await?
        .watch()
        .await?;
    contract2
        .setValue("contract2_msg2".to_string())
        .send()
        .await?
        .watch()
        .await?;

    EvmEventReader::attach(
        provider.clone(),
        test_event_extractor,
        &contract1.address().to_string(),
        None,
        &processor,
        &bus,
        &repository1,
        rpc_url.clone(),
    )
    .await?;

    EvmEventReader::attach(
        provider.clone(),
        test_event_extractor,
        &contract2.address().to_string(),
        None,
        &processor,
        &bus,
        &repository2,
        rpc_url.clone(),
    )
    .await?;

    coordinator.do_send(CoordinatorStart);

    // Wait for historical events to be processed
    sleep(Duration::from_millis(200)).await;

    let msgs = get_msgs(&history_collector).await?;
    assert_eq!(msgs.len(), 4);
    assert!(msgs.contains(&"contract1_msg1".to_string()));
    assert!(msgs.contains(&"contract2_msg1".to_string()));
    assert!(msgs.contains(&"contract1_msg2".to_string()));
    assert!(msgs.contains(&"contract2_msg2".to_string()));

    Ok(())
}

/// Verifies that an EVM event reader attached with no historical events processes only subsequently emitted (live) events.
///
/// # Examples
///
/// ```
/// // When no historical events are present, the history starts empty and
/// // later contains events emitted after the reader is attached.
/// let msgs_before: Vec<&str> = vec![];
/// assert_eq!(msgs_before.len(), 0);
///
/// let msgs_after = vec!["live1", "live2"];
/// assert_eq!(msgs_after, ["live1", "live2"]);
/// ```
#[actix::test]
async fn coordinator_no_historical_events() -> Result<()> {
    let anvil = Anvil::new().block_time(1).try_spawn()?;
    let rpc_url = anvil.ws_endpoint();
    let provider = EthProvider::new(
        ProviderBuilder::new()
            .wallet(PrivateKeySigner::from_slice(&anvil.keys()[0].to_bytes())?)
            .connect_ws(WsConnect::new(rpc_url.clone()))
            .await?,
    )
    .await?;
    let contract = EmitLogs::deploy(provider.provider()).await?;
    let system = EventSystem::new("test").with_fresh_bus();
    let bus = system.handle()?;
    let history_collector = bus.history();
    let repository = Repository::new(get_in_mem_store());

    let coordinator = HistoricalEventCoordinator::setup(bus.clone());
    let processor = coordinator.clone().recipient();

    EvmEventReader::attach(
        provider.clone(),
        test_event_extractor,
        &contract.address().to_string(),
        None,
        &processor,
        &bus,
        &repository,
        rpc_url.clone(),
    )
    .await?;

    coordinator.do_send(CoordinatorStart);
    sleep(Duration::from_millis(50)).await;

    let msgs = get_msgs(&history_collector).await?;
    assert_eq!(msgs.len(), 0);

    for msg in ["live1", "live2"] {
        contract
            .setValue(msg.to_string())
            .send()
            .await?
            .watch()
            .await?;
    }

    sleep(Duration::from_millis(100)).await;
    let msgs = get_msgs(&history_collector).await?;
    assert_eq!(msgs, ["live1", "live2"]);

    Ok(())
}