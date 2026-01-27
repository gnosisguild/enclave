// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Addr, Handler};
use alloy::{
    node_bindings::Anvil,
    primitives::{FixedBytes, LogData},
    providers::{ProviderBuilder, WsConnect},
    signers::local::PrivateKeySigner,
    sol,
    sol_types::SolEvent,
};
use anyhow::Result;
use e3_ciphernode_builder::{EventSystem, EvmSystemChainBuilder};
use e3_events::{
    prelude::*, trap, BusHandle, EType, EnclaveEvent, EnclaveEventData, EvmEvent, EvmEventConfig,
    EvmEventConfigChain, GetEvents, HistoryCollector, SyncEnd, SyncEvmEvent, SyncStart, TestEvent,
};
use e3_evm::{helpers::EthProvider, EvmEventProcessor, EvmReader};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::subscriber::DefaultGuard;
use tracing_subscriber::{fmt, EnvFilter};

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

struct TestEventParser;

impl TestEventParser {
    pub fn setup(next: &EvmEventProcessor) -> Addr<EvmReader> {
        EvmReader::new(next, test_event_extractor).start()
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

struct FakeSyncActor {
    bus: BusHandle,
    buffer: Vec<EvmEvent>,
}

impl Actor for FakeSyncActor {
    type Context = actix::Context<Self>;
}

impl FakeSyncActor {
    pub fn setup(bus: &BusHandle) -> Addr<Self> {
        Self {
            bus: bus.clone(),
            buffer: Vec::new(),
        }
        .start()
    }
}

impl Handler<SyncEvmEvent> for FakeSyncActor {
    type Result = ();
    fn handle(&mut self, msg: SyncEvmEvent, ctx: &mut Self::Context) -> Self::Result {
        trap(EType::Sync, &self.bus.clone(), || {
            match msg {
                // Buffer events as the sync actor receives them
                SyncEvmEvent::Event(event) => self.buffer.push(event),
                // When we hear that sync is complete send all events on chain then publish SyncEnd
                SyncEvmEvent::HistoricalSyncComplete(_) => {
                    for evt in self.buffer.drain(..) {
                        let (data, ts, _) = evt.split();
                        self.bus.publish_from_remote(data, ts)?;
                    }
                    self.bus.publish(SyncEnd::new())?;
                }
            };
            Ok(())
        })
    }
}

fn add_tracing() -> DefaultGuard {
    tracing::subscriber::set_default(
        fmt()
            .with_env_filter(EnvFilter::new("info"))
            .with_test_writer()
            .finish(),
    )
}

#[actix::test]
async fn evm_reader() -> Result<()> {
    let _guard = add_tracing();

    // Create a WS provider
    // NOTE: Anvil must be available on $PATH
    let anvil = Anvil::new().block_time(1).try_spawn()?;
    let rpc_url = anvil.ws_endpoint(); // Get RPC URL
    let provider = Arc::new(
        EthProvider::new(
            ProviderBuilder::new()
                .wallet(PrivateKeySigner::from_slice(&anvil.keys()[0].to_bytes())?)
                .connect_ws(WsConnect::new(rpc_url.clone())) // Use RPC URL
                .await?,
        )
        .await?,
    );
    let contract = EmitLogs::deploy(provider.provider()).await?;
    let system = EventSystem::new("test").with_fresh_bus();
    let bus = system.handle()?;
    let history_collector = bus.history();

    let chain_id = provider.chain_id();
    let contract_address = contract.address().clone();
    let sync = FakeSyncActor::setup(&bus);
    EvmSystemChainBuilder::new(&bus, &provider)
        .with_contract(contract_address, move |upstream| {
            TestEventParser::setup(&upstream).recipient()
        })
        .build();

    // SyncStart holds initialization information such as start block and earliest event
    // This should trigger all chains to start to sync
    let mut evm_info = EvmEventConfig::new();
    evm_info.insert(chain_id, EvmEventConfigChain::new(0));
    bus.publish(SyncStart::new(sync, evm_info))?;

    sleep(Duration::from_secs(1)).await;
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

    sleep(Duration::from_secs(1)).await;

    let history = history_collector
        .send(GetEvents::<EnclaveEvent>::new())
        .await?;

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
#[actix::test]
async fn ensure_historical_events() -> Result<()> {
    let _guard = add_tracing();

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
    let contract_address = contract.address().clone();
    let chain_id = provider.chain_id();
    let system = EventSystem::new("test").with_fresh_bus();
    let bus = system.handle()?;
    let history_collector = bus.history();
    let historical_msgs = vec!["these", "are", "historical", "events"];
    let live_events = vec!["these", "events", "are", "live"];

    for msg in historical_msgs.clone() {
        contract
            .setValue(msg.to_string())
            .send()
            .await?
            .watch()
            .await?;
    }

    sleep(Duration::from_millis(1)).await;

    let sync = FakeSyncActor::setup(&bus);
    EvmSystemChainBuilder::new(&bus, &provider)
        .with_contract(contract_address, move |upstream| {
            TestEventParser::setup(&upstream).recipient()
        })
        .build();
    let mut evm_info = EvmEventConfig::new();
    evm_info.insert(chain_id, EvmEventConfigChain::new(0));
    bus.publish(SyncStart::new(sync, evm_info))?;

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
