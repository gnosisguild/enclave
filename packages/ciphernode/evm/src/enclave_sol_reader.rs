use std::collections::HashSet;

use crate::event_reader::{EnclaveEvmEvent, EventReader};
use crate::helpers::{ReadonlyProvider, WithChainId};
use crate::EvmEventReader;
use actix::{Actor, Addr, Handler};
use alloy::primitives::{LogData, B256};
use alloy::transports::BoxTransport;
use alloy::{sol, sol_types::SolEvent};
use anyhow::Result;
use async_trait::async_trait;
use data::{Checkpoint, FromSnapshotWithParams, Repository, Snapshot};
use enclave_core::{EnclaveEvent, EventBus, EventId};
use tracing::{error, info, trace};

sol!(
    #[sol(rpc)]
    IEnclave,
    "../../evm/artifacts/contracts/interfaces/IEnclave.sol/IEnclave.json"
);

struct E3RequestedWithChainId(pub IEnclave::E3Requested, pub u64);

impl From<E3RequestedWithChainId> for enclave_core::E3Requested {
    fn from(value: E3RequestedWithChainId) -> Self {
        enclave_core::E3Requested {
            params: value.0.e3.e3ProgramParams.to_vec(),
            threshold_m: value.0.e3.threshold[0] as usize,
            seed: value.0.e3.seed.into(),
            e3_id: value.0.e3Id.to_string().into(),
            src_chain_id: value.1,
        }
    }
}

impl From<E3RequestedWithChainId> for EnclaveEvent {
    fn from(value: E3RequestedWithChainId) -> Self {
        let payload: enclave_core::E3Requested = value.into();
        EnclaveEvent::from(payload)
    }
}

impl From<IEnclave::CiphertextOutputPublished> for enclave_core::CiphertextOutputPublished {
    fn from(value: IEnclave::CiphertextOutputPublished) -> Self {
        enclave_core::CiphertextOutputPublished {
            e3_id: value.e3Id.to_string().into(),
            ciphertext_output: value.ciphertextOutput.to_vec(),
        }
    }
}

impl From<IEnclave::CiphertextOutputPublished> for EnclaveEvent {
    fn from(value: IEnclave::CiphertextOutputPublished) -> Self {
        let payload: enclave_core::CiphertextOutputPublished = value.into();
        EnclaveEvent::from(payload)
    }
}

pub fn extractor(data: &LogData, topic: Option<&B256>, chain_id: u64) -> Option<EnclaveEvent> {
    match topic {
        Some(&IEnclave::E3Requested::SIGNATURE_HASH) => {
            let Ok(event) = IEnclave::E3Requested::decode_log_data(data, true) else {
                error!("Error parsing event E3Requested after topic matched!");
                return None;
            };
            Some(EnclaveEvent::from(E3RequestedWithChainId(event, chain_id)))
        }
        Some(&IEnclave::CiphertextOutputPublished::SIGNATURE_HASH) => {
            let Ok(event) = IEnclave::CiphertextOutputPublished::decode_log_data(data, true) else {
                error!("Error parsing event CiphertextOutputPublished after topic matched!"); // TODO: provide more info
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

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct EnclaveSolReaderState {
    pub ids: HashSet<EventId>,
    pub last_block: Option<u64>,
}

impl Default for EnclaveSolReaderState {
    fn default() -> Self {
        Self {
            ids: HashSet::new(),
            last_block: None,
        }
    }
}

/// Connects to Enclave.sol converting EVM events to EnclaveEvents
pub struct EnclaveSolReader {
    bus: Addr<EventBus>,
    state: EnclaveSolReaderState,
    repository: Repository<EnclaveSolReaderState>,
}

pub struct EnclaveSolReaderParams {
    bus: Addr<EventBus>,
    repository: Repository<EnclaveSolReaderState>,
}

impl EnclaveSolReader {
    pub fn new(params: EnclaveSolReaderParams) -> Self {
        Self {
            bus: params.bus,
            state: EnclaveSolReaderState::default(),
            repository: params.repository,
        }
    }

    pub async fn load(params: EnclaveSolReaderParams) -> Result<Self> {
        Ok(if let Some(snapshot) = params.repository.read().await? {
            Self::from_snapshot(params, snapshot).await?
        } else {
            Self::new(params)
        })
    }

    pub async fn attach(
        bus: &Addr<EventBus>,
        provider: &WithChainId<ReadonlyProvider, BoxTransport>,
        contract_address: &str,
        repository: &Repository<EnclaveSolReaderState>,
    ) -> Result<Addr<Self>> {
        let params = EnclaveSolReaderParams {
            bus: bus.clone(),
            repository: repository.clone(),
        };

        let actor = Self::load(params).await?;
        let last_block = actor.state.last_block;
        let addr = actor.start();

        EvmEventReader::attach(
            &addr.clone().recipient(),
            provider,
            extractor,
            contract_address,
            last_block,
            &bus.clone(),
        )
        .await?;

        info!(address=%contract_address, "EnclaveSolReader is listening to address");

        Ok(addr)
    }
}

impl Actor for EnclaveSolReader {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvmEvent> for EnclaveSolReader {
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

impl Snapshot for EnclaveSolReader {
    type Snapshot = EnclaveSolReaderState;
    fn snapshot(&self) -> Self::Snapshot {
        self.state.clone()
    }
}

impl Checkpoint for EnclaveSolReader {
    fn repository(&self) -> &Repository<Self::Snapshot> {
        &self.repository
    }
}

#[async_trait]
impl FromSnapshotWithParams for EnclaveSolReader {
    type Params = EnclaveSolReaderParams;
    async fn from_snapshot(params: Self::Params, snapshot: Self::Snapshot) -> Result<Self> {
        Ok(Self {
            bus: params.bus,
            state: snapshot,
            repository: params.repository,
        })
    }
}
