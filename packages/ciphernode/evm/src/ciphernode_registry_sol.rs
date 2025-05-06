use crate::{
    event_reader::EvmEventReaderState,
    helpers::{ReadonlyProvider, WithChainId},
    EvmEventReader,
};
use actix::Addr;
use alloy::{
    primitives::{LogData, B256},
    sol,
    sol_types::SolEvent,
    transports::BoxTransport,
};
use anyhow::Result;
use data::Repository;
use events::{EnclaveEvent, EventBus};
use tracing::{error, info, trace};

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    ICiphernodeRegistry,
    "../../evm/artifacts/contracts/interfaces/ICiphernodeRegistry.sol/ICiphernodeRegistry.json"
);

struct CiphernodeAddedWithChainId(pub ICiphernodeRegistry::CiphernodeAdded, pub u64);

impl From<CiphernodeAddedWithChainId> for events::CiphernodeAdded {
    fn from(value: CiphernodeAddedWithChainId) -> Self {
        events::CiphernodeAdded {
            address: value.0.node.to_string(),
            // TODO: limit index and numNodes to uint32 at the solidity level
            index: value
                .0
                .index
                .try_into()
                .expect("Index exceeds usize capacity"),
            num_nodes: value
                .0
                .numNodes
                .try_into()
                .expect("NumNodes exceeds usize capacity"),
            chain_id: value.1,
        }
    }
}

impl From<CiphernodeAddedWithChainId> for EnclaveEvent {
    fn from(value: CiphernodeAddedWithChainId) -> Self {
        let payload: events::CiphernodeAdded = value.into();
        EnclaveEvent::from(payload)
    }
}

struct CiphernodeRemovedWithChainId(pub ICiphernodeRegistry::CiphernodeRemoved, pub u64);
impl From<CiphernodeRemovedWithChainId> for events::CiphernodeRemoved {
    fn from(value: CiphernodeRemovedWithChainId) -> Self {
        events::CiphernodeRemoved {
            address: value.0.node.to_string(),
            index: value
                .0
                .index
                .try_into()
                .expect("Index exceeds usize capacity"),
            num_nodes: value
                .0
                .numNodes
                .try_into()
                .expect("NumNodes exceeds usize capacity"),
            chain_id: value.1,
        }
    }
}

impl From<CiphernodeRemovedWithChainId> for EnclaveEvent {
    fn from(value: CiphernodeRemovedWithChainId) -> Self {
        let payload: events::CiphernodeRemoved = value.into();
        EnclaveEvent::from(payload)
    }
}

pub fn extractor(data: &LogData, topic: Option<&B256>, chain_id: u64) -> Option<EnclaveEvent> {
    match topic {
        Some(&ICiphernodeRegistry::CiphernodeAdded::SIGNATURE_HASH) => {
            let Ok(event) = ICiphernodeRegistry::CiphernodeAdded::decode_log_data(data, true)
            else {
                error!("Error parsing event CiphernodeAdded after topic was matched!");
                return None;
            };
            Some(EnclaveEvent::from(CiphernodeAddedWithChainId(
                event, chain_id,
            )))
        }
        Some(&ICiphernodeRegistry::CiphernodeRemoved::SIGNATURE_HASH) => {
            let Ok(event) = ICiphernodeRegistry::CiphernodeRemoved::decode_log_data(data, true)
            else {
                error!("Error parsing event CiphernodeRemoved after topic was matched!");
                return None;
            };
            Some(EnclaveEvent::from(CiphernodeRemovedWithChainId(
                event, chain_id,
            )))
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
pub struct CiphernodeRegistrySolReader;
impl CiphernodeRegistrySolReader {
    pub async fn attach(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: &WithChainId<ReadonlyProvider, BoxTransport>,
        contract_address: &str,
        repository: &Repository<EvmEventReaderState>,
        start_block: Option<u64>,
    ) -> Result<Addr<EvmEventReader<ReadonlyProvider>>> {
        let addr = EvmEventReader::attach(
            provider,
            extractor,
            contract_address,
            start_block,
            &bus.clone().into(),
            repository,
        )
        .await?;

        info!(address=%contract_address, "EnclaveSolReader is listening to address");

        Ok(addr)
    }
}

/// Wrapper for a reader and a future writer
pub struct CiphernodeRegistrySol;
impl CiphernodeRegistrySol {
    pub async fn attach(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: &WithChainId<ReadonlyProvider, BoxTransport>,
        contract_address: &str,
        repository: &Repository<EvmEventReaderState>,
        start_block: Option<u64>,
    ) -> Result<()> {
        CiphernodeRegistrySolReader::attach(
            bus,
            provider,
            contract_address,
            repository,
            start_block,
        )
        .await?;
        // TODO: Writer if needed
        Ok(())
    }
}
