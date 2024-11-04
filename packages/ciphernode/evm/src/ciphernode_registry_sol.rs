use std::collections::HashSet;

use crate::{
    event_reader::{EnclaveEvmEvent, EventReader},
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
    state: CiphernodeRegistryReaderState,
    repository: Repository<CiphernodeRegistryReaderState>,
}

pub struct CiphernodeRegistryReaderParams {
    bus: Addr<EventBus>,
    repository: Repository<CiphernodeRegistryReaderState>,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CiphernodeRegistryReaderState {
    pub ids: HashSet<EventId>,
    pub last_block: Option<u64>,
}

impl CiphernodeRegistrySolReader {
    pub fn new(params: CiphernodeRegistryReaderParams) -> Self {
        Self {
            bus: params.bus,
            state: CiphernodeRegistryReaderState::default(),
            repository: params.repository,
        }
    }

    pub async fn load(params: CiphernodeRegistryReaderParams) -> Result<Self> {
        Ok(if let Some(snapshot) = params.repository.read().await? {
            Self::from_snapshot(params, snapshot).await?
        } else {
            Self::new(params)
        })
    }

    pub async fn attach(
        bus: &Addr<EventBus>,
        provider: &WithChainId<ReadonlyProvider>,
        contract_address: &str,
        repository: &Repository<CiphernodeRegistryReaderState>,
    ) -> Result<Addr<Self>> {
        let params = CiphernodeRegistryReaderParams {
            bus: bus.clone(),
            repository: repository.clone(),
        };

        let actor = Self::load(params).await?;
        let last_block = actor.state.last_block;
        let addr = actor.start();

        EvmEventReader::attach(
            &addr.clone().into(),
            provider,
            extractor,
            contract_address,
            last_block,
            &bus.clone().into(),
        )
        .await?;

        info!(address=%contract_address, "EnclaveSolReader is listening to address");

        Ok(addr)
    }
}

impl Actor for CiphernodeRegistrySolReader {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvmEvent> for CiphernodeRegistrySolReader {
    type Result = ();
    fn handle(&mut self, wrapped: EnclaveEvmEvent, _: &mut Self::Context) -> Self::Result {
        let event_id = wrapped.event.get_id();
        if self.state.ids.contains(&event_id) {
            trace!(
                "Event id {} has already been seen and was not forwarded to the bus",
                &event_id
            );
            return;
        }

        // Forward everything else to the event bus
        self.bus.do_send(wrapped.event);

        // Save processed ids
        self.state.ids.insert(event_id);
        self.state.last_block = wrapped.block;
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
        repository: &Repository<CiphernodeRegistryReaderState>,
    ) -> Result<()> {
        CiphernodeRegistrySolReader::attach(bus, provider, contract_address, repository).await?;
        // TODO: Writer if needed
        Ok(())
    }
}
