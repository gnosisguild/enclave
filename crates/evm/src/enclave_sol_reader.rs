use crate::event_reader::EvmEventReaderState;
use crate::helpers::EthProvider;
use crate::EvmEventReader;
use actix::Addr;
use alloy::primitives::{LogData, B256};
use alloy::providers::Provider;
use alloy::{sol, sol_types::SolEvent};
use anyhow::Result;
use e3_data::Repository;
use e3_events::{E3id, EnclaveEvent, EventBus};
use tracing::{error, info, trace};

sol!(
    #[sol(rpc)]
    IEnclave,
    "../../packages/evm/artifacts/contracts/interfaces/IEnclave.sol/IEnclave.json"
);

struct E3RequestedWithChainId(pub IEnclave::E3Requested, pub u64);

impl From<E3RequestedWithChainId> for e3_events::E3Requested {
    fn from(value: E3RequestedWithChainId) -> Self {
        e3_events::E3Requested {
            params: value.0.e3.e3ProgramParams.to_vec(),
            threshold_m: value.0.e3.threshold[0] as usize,
            seed: value.0.e3.seed.into(),
            e3_id: E3id::new(value.0.e3Id.to_string(), value.1),
        }
    }
}

impl From<E3RequestedWithChainId> for EnclaveEvent {
    fn from(value: E3RequestedWithChainId) -> Self {
        let payload: e3_events::E3Requested = value.into();
        EnclaveEvent::from(payload)
    }
}

struct CiphertextOutputPublishedWithChainId(pub IEnclave::CiphertextOutputPublished, pub u64);

impl From<CiphertextOutputPublishedWithChainId> for e3_events::CiphertextOutputPublished {
    fn from(value: CiphertextOutputPublishedWithChainId) -> Self {
        e3_events::CiphertextOutputPublished {
            e3_id: E3id::new(value.0.e3Id.to_string(), value.1),
            ciphertext_output: value.0.ciphertextOutput.to_vec(),
        }
    }
}

impl From<CiphertextOutputPublishedWithChainId> for EnclaveEvent {
    fn from(value: CiphertextOutputPublishedWithChainId) -> Self {
        let payload: e3_events::CiphertextOutputPublished = value.into();
        EnclaveEvent::from(payload)
    }
}

pub fn extractor(data: &LogData, topic: Option<&B256>, chain_id: u64) -> Option<EnclaveEvent> {
    match topic {
        Some(&IEnclave::E3Requested::SIGNATURE_HASH) => {
            let Ok(event) = IEnclave::E3Requested::decode_log_data(data) else {
                error!("Error parsing event E3Requested after topic matched!");
                return None;
            };
            Some(EnclaveEvent::from(E3RequestedWithChainId(event, chain_id)))
        }
        Some(&IEnclave::CiphertextOutputPublished::SIGNATURE_HASH) => {
            let Ok(event) = IEnclave::CiphertextOutputPublished::decode_log_data(data) else {
                error!("Error parsing event CiphertextOutputPublished after topic matched!");
                return None;
            };
            Some(EnclaveEvent::from(CiphertextOutputPublishedWithChainId(
                event, chain_id,
            )))
        }
        _topic => {
            trace!(
                topic=?_topic,
                "Unknown event received by Enclave.sol parser but was ignored"
            );
            None
        }
    }
}

/// Connects to Enclave.sol converting EVM events to EnclaveEvents
pub struct EnclaveSolReader;

impl EnclaveSolReader {
    pub async fn attach<P>(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: EthProvider<P>,
        contract_address: &str,
        repository: &Repository<EvmEventReaderState>,
        start_block: Option<u64>,
        rpc_url: String,
    ) -> Result<Addr<EvmEventReader<P>>>
    where
        P: Provider + Clone + 'static,
    {
        let addr = EvmEventReader::attach(
            provider,
            extractor,
            contract_address,
            start_block,
            &bus.clone(),
            repository,
            rpc_url,
        )
        .await?;

        info!(address=%contract_address, "EnclaveSolReader is listening to address");

        Ok(addr)
    }
}