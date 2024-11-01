use crate::{helpers::{ReadonlyProvider, WithChainId}, EvmEventReader};
use actix::Addr;
use alloy::{
    primitives::{LogData, B256},
    sol,
    sol_types::SolEvent,
};
use anyhow::Result;
use enclave_core::{EnclaveEvent, EventBus};
use tracing::{error, trace};

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
pub struct CiphernodeRegistrySolReader;

impl CiphernodeRegistrySolReader {
    pub async fn attach(
        bus: &Addr<EventBus>,
        provider: &WithChainId<ReadonlyProvider>,
        contract_address: &str,
    ) -> Result<Addr<EvmEventReader<ReadonlyProvider>>> {
        let addr = EvmEventReader::attach(bus, provider, extractor, contract_address).await?;
        Ok(addr)
    }
}

pub struct CiphernodeRegistrySol;
impl CiphernodeRegistrySol {
    pub async fn attach(
        bus: &Addr<EventBus>,
        provider: &WithChainId<ReadonlyProvider>,
        contract_address: &str,
    ) -> Result<()> {
        CiphernodeRegistrySolReader::attach(bus, provider, contract_address).await?;
        Ok(())
    }
}
