// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::event_reader::EvmEventReaderState;
use crate::helpers::EthProvider;
use crate::{EnclaveEvmEvent, EvmEventReader};
use actix::{Addr, Recipient};
use alloy::primitives::{LogData, B256};
use alloy::providers::Provider;
use alloy::{sol, sol_types::SolEvent};
use anyhow::Result;
use e3_data::Repository;
use e3_events::{prelude::*, BusHandle, E3id, EnclaveEvent, EnclaveEventData};
use e3_utils::utility_types::ArcBytes;
use num_bigint::BigUint;
use tracing::{error, info, trace};

sol!(
    #[sol(rpc)]
    IEnclave,
    "../../packages/enclave-contracts/artifacts/contracts/interfaces/IEnclave.sol/IEnclave.json"
);

struct E3RequestedWithChainId(pub IEnclave::E3Requested, pub u64);

impl From<E3RequestedWithChainId> for e3_events::E3Requested {
    fn from(value: E3RequestedWithChainId) -> Self {
        e3_events::E3Requested {
            params: ArcBytes::from_bytes(&value.0.e3.e3ProgramParams.to_vec()),
            threshold_m: value.0.e3.threshold[0] as usize,
            threshold_n: value.0.e3.threshold[1] as usize,
            seed: value.0.e3.seed.into(),
            // TODO: this should be delivered from the e3_program. Here we provide a sensible
            // default that passes our tests
            error_size: ArcBytes::from_bytes(
                &BigUint::from(36128399948547143872891754381312u128).to_bytes_be(),
            ),
            esi_per_ct: 3, // TODO: this should be delivered from the e3_program
            e3_id: E3id::new(value.0.e3Id.to_string(), value.1),
        }
    }
}

impl From<E3RequestedWithChainId> for EnclaveEventData {
    fn from(value: E3RequestedWithChainId) -> Self {
        let payload: e3_events::E3Requested = value.into();
        payload.into()
    }
}

struct CiphertextOutputPublishedWithChainId(pub IEnclave::CiphertextOutputPublished, pub u64);

impl From<CiphertextOutputPublishedWithChainId> for e3_events::CiphertextOutputPublished {
    fn from(value: CiphertextOutputPublishedWithChainId) -> Self {
        e3_events::CiphertextOutputPublished {
            e3_id: E3id::new(value.0.e3Id.to_string(), value.1),
            // XXX: Ciphertext is an array of bytes this needs to be coordinated with enclave
            // contract
            ciphertext_output: vec![ArcBytes::from_bytes(&value.0.ciphertextOutput.to_vec())],
        }
    }
}

impl From<CiphertextOutputPublishedWithChainId> for EnclaveEventData {
    fn from(value: CiphertextOutputPublishedWithChainId) -> Self {
        let payload: e3_events::CiphertextOutputPublished = value.into();
        payload.into()
    }
}

pub fn extractor(data: &LogData, topic: Option<&B256>, chain_id: u64) -> Option<EnclaveEventData> {
    match topic {
        Some(&IEnclave::E3Requested::SIGNATURE_HASH) => {
            let Ok(event) = IEnclave::E3Requested::decode_log_data(data) else {
                error!("Error parsing event E3Requested after topic matched!");
                return None;
            };
            Some(EnclaveEventData::from(E3RequestedWithChainId(
                event, chain_id,
            )))
        }
        Some(&IEnclave::CiphertextOutputPublished::SIGNATURE_HASH) => {
            let Ok(event) = IEnclave::CiphertextOutputPublished::decode_log_data(data) else {
                error!("Error parsing event CiphertextOutputPublished after topic matched!");
                return None;
            };
            Some(EnclaveEventData::from(
                CiphertextOutputPublishedWithChainId(event, chain_id),
            ))
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
        processor: &Recipient<EnclaveEvmEvent>,
        bus: &BusHandle<EnclaveEvent>,
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
            processor,
            bus,
            repository,
            rpc_url,
        )
        .await?;

        info!(address=%contract_address, "EnclaveSolReader is listening to address");

        Ok(addr)
    }
}
