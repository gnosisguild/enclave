use std::collections::HashSet;

use crate::{
    event_reader::EventReader,
    helpers::{ReadonlyProvider, WithChainId},
     EvmEventReader,
};
use actix::{Actor, Addr, Handler};
use alloy::{
    primitives::{LogData, B256},
    sol,
    sol_types::SolEvent,
};
use anyhow::Result;
use async_trait::async_trait;
use data::{Checkpoint, FromSnapshotWithParams, Repository, Snapshot};
use enclave_core::{EnclaveEvent, EventBus, EventId, Subscribe};
use tracing::{error, info, trace};

sol!(
    #[sol(rpc)]
    ICiphernodeRegistry,
    "../../evm/artifacts/contracts/interfaces/ICiphernodeRegistry.sol/ICiphernodeRegistry.json"
);

impl From<ICiphernodeRegistry::CiphernodeAdded> for enclave_core::CiphernodeAdded {
    fn from(value: ICiphernodeRegistry::CiphernodeAdded) -> Self {
        enclave_core::CiphernodeAdded {
            address: value.node.to_string(),
            // TODO: limit index and numNodes to uint32 at the solidity level
            index: value
                .index
                .try_into()
                .expect("Index exceeds usize capacity"),
            num_nodes: value
                .numNodes
                .try_into()
                .expect("NumNodes exceeds usize capacity"),
        }
    }
}

impl From<ICiphernodeRegistry::CiphernodeAdded> for EnclaveEvent {
    fn from(value: ICiphernodeRegistry::CiphernodeAdded) -> Self {
        let payload: enclave_core::CiphernodeAdded = value.into();
        EnclaveEvent::from(payload)
    }
}

impl From<ICiphernodeRegistry::CiphernodeRemoved> for enclave_core::CiphernodeRemoved {
    fn from(value: ICiphernodeRegistry::CiphernodeRemoved) -> Self {
        enclave_core::CiphernodeRemoved {
            address: value.node.to_string(),
            index: value
                .index
                .try_into()
                .expect("Index exceeds usize capacity"),
            num_nodes: value
                .numNodes
                .try_into()
                .expect("NumNodes exceeds usize capacity"),
        }
    }
}

impl From<ICiphernodeRegistry::CiphernodeRemoved> for EnclaveEvent {
    fn from(value: ICiphernodeRegistry::CiphernodeRemoved) -> Self {
        let payload: enclave_core::CiphernodeRemoved = value.into();
        EnclaveEvent::from(payload)
    }
}

pub fn extractor(data: &LogData, topic: Option<&B256>, _: u64) -> Option<EnclaveEvent> {
    match topic {
        Some(&ICiphernodeRegistry::CiphernodeAdded::SIGNATURE_HASH) => {
            let Ok(event) = ICiphernodeRegistry::CiphernodeAdded::decode_log_data(data, true)
            else {
                error!("Error parsing event CiphernodeAdded after topic was matched!");
                return None;
            };
            Some(EnclaveEvent::from(event))
        }
        Some(&ICiphernodeRegistry::CiphernodeRemoved::SIGNATURE_HASH) => {
            let Ok(event) = ICiphernodeRegistry::CiphernodeRemoved::decode_log_data(data, true)
            else {
                error!("Error parsing event CiphernodeRemoved after topic was matched!");
                return None;
            };
            Some(EnclaveEvent::from(event))
        }

        _topic => {
            trace!(
                topic=?_topic,
                "Unknown event was received by Enclave.sol parser buut was ignored"
            );
            return None;
        }
    }
}

/// Connects to CiphernodeRegistry.sol converting EVM events to EnclaveEvents
pub struct CiphernodeRegistrySolReader {
    bus: Addr<EventBus>,
    reader: Addr<EventReader>,
    state: CiphernodeRegistryReaderState,
    repository: Repository<CiphernodeRegistryReaderState>,
}

pub struct CiphernodeRegistryReaderParams {
    bus: Addr<EventBus>,
    reader: Addr<EventReader>,
    repository: Repository<CiphernodeRegistryReaderState>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct CiphernodeRegistryReaderState {
    ids: HashSet<EventId>,
}

impl CiphernodeRegistrySolReader {
    pub fn new(params: CiphernodeRegistryReaderParams) -> Self {
        Self {
            bus: params.bus,
            reader: params.reader,
            state: CiphernodeRegistryReaderState {
                ids: HashSet::new(),
            },
            repository: params.repository,
        }
    }

    pub async fn load(params: CiphernodeRegistryReaderParams) -> Result<Addr<Self>> {
        let addr = if let Some(snapshot) = params.repository.read().await? {
            Self::from_snapshot(params, snapshot).await?
        } else {
            Self::new(params)
        }
        .start();
        Ok(addr)
    }

    pub async fn attach(
        bus: &Addr<EventBus>,
        provider: &WithChainId<ReadonlyProvider>,
        contract_address: &str,
        repository: &Repository<CiphernodeRegistryReaderState>,
    ) -> Result<Addr<Self>> {
        let params = CiphernodeRegistryReaderParams {
            bus: bus.clone(),
            reader: EvmEventReader::attach(
                &bus.clone().into(),
                provider,
                extractor,
                contract_address,
                None,
            )
            .await?,
            repository: repository.clone(),
        };

        let addr = Self::load(params).await?;

        bus.send(Subscribe::new("Shutdown", addr.clone().into()))
            .await?;

        info!(address=%contract_address, "EnclaveSolReader is listening to address");

        Ok(addr)
    }
}

impl Actor for CiphernodeRegistrySolReader {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for CiphernodeRegistrySolReader {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        // If this is a shutdown signal it will be coming from the event bus forward it to the reader
        if let EnclaveEvent::Shutdown { .. } = msg {
            self.reader.do_send(msg);
            return;
        }

        // Other enclave events will be coming from the reader - check the event id cache forward to the event bus
        let event_id = msg.get_id();
        if self.state.ids.contains(&event_id) {
            trace!(
                "Event id {} has already been seen and was not forwarded to the bus",
                &event_id
            );
            return;
        }

        // Forward everything else to the event bus
        self.bus.do_send(msg);

        // Save processed ids
        self.state.ids.insert(event_id);
        self.checkpoint();
    }
}

impl Snapshot for CiphernodeRegistrySolReader {
    type Snapshot = CiphernodeRegistryReaderState;
    fn snapshot(&self) -> Self::Snapshot {
        self.state.clone()
    }
}

impl Checkpoint for CiphernodeRegistrySolReader {
    fn repository(&self) -> &Repository<Self::Snapshot> {
        &self.repository
    }
}

#[async_trait]
impl FromSnapshotWithParams for CiphernodeRegistrySolReader {
    type Params = CiphernodeRegistryReaderParams;
    async fn from_snapshot(params: Self::Params, snapshot: Self::Snapshot) -> Result<Self> {
        Ok(Self {
            bus: params.bus,
            reader: params.reader,
            state: snapshot,
            repository: params.repository,
        })
    }
}
/// Eventual wrapper for both a reader and a writer
pub struct CiphernodeRegistrySol;
impl CiphernodeRegistrySol {
    pub async fn attach(
        bus: &Addr<EventBus>,
        provider: &WithChainId<ReadonlyProvider>,
        contract_address: &str,
        repository: &Repository<CiphernodeRegistryReaderState>
    ) -> Result<()> {
        CiphernodeRegistrySolReader::attach(bus, provider, contract_address,repository).await?;
        // TODO: Writer if needed
        Ok(())
    }
}
