use crate::event_reader::EvmEventReaderState;
use crate::helpers::{ReadonlyProvider, WithChainId};
use crate::EvmEventReader;
use actix::Addr;
use alloy::primitives::{LogData, B256};
use alloy::transports::BoxTransport;
use alloy::{sol, sol_types::SolEvent};
use anyhow::Result;
use data::Repository;
use events::{EnclaveEvent, EventBus};
use tracing::{error, info, trace};

sol!(
    #[sol(rpc)]
    IEnclave,
    "../../evm/artifacts/contracts/interfaces/IEnclave.sol/IEnclave.json"
);

struct E3RequestedWithChainId(pub IEnclave::E3Requested, pub u64);

impl From<E3RequestedWithChainId> for events::E3Requested {
    fn from(value: E3RequestedWithChainId) -> Self {
        events::E3Requested {
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
        let payload: events::E3Requested = value.into();
        EnclaveEvent::from(payload)
    }
}

impl From<IEnclave::CiphertextOutputPublished> for events::CiphertextOutputPublished {
    fn from(value: IEnclave::CiphertextOutputPublished) -> Self {
        events::CiphertextOutputPublished {
            e3_id: value.e3Id.to_string().into(),
            ciphertext_output: value.ciphertextOutput.to_vec(),
        }
    }
}

impl From<IEnclave::CiphertextOutputPublished> for EnclaveEvent {
    fn from(value: IEnclave::CiphertextOutputPublished) -> Self {
        let payload: events::CiphertextOutputPublished = value.into();
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

/// Connects to Enclave.sol converting EVM events to EnclaveEvents
pub struct EnclaveSolReader;

impl EnclaveSolReader {
    pub async fn attach(
        bus: &Addr<EventBus>,
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
            &bus.clone(),
            repository,
        )
        .await?;

        info!(address=%contract_address, "EnclaveSolReader is listening to address");

        Ok(addr)
    }
}
