use crate::helpers::{self, create_readonly_provider, ReadonlyProvider};
use actix::prelude::*;
use actix::{Addr, Recipient};
use alloy::primitives::{LogData, B256};
use alloy::{
    eips::BlockNumberOrTag, primitives::Address, rpc::types::Filter, sol, sol_types::SolEvent,
};
use anyhow::Result;
use enclave_core::{EnclaveEvent, EventBus};

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

fn extractor(data: &LogData, topic: Option<&B256>, chain_id: u64) -> Option<EnclaveEvent> {
    match topic {
        Some(&IEnclave::E3Requested::SIGNATURE_HASH) => {
            let Ok(event) = IEnclave::E3Requested::decode_log_data(data, true) else {
                println!("Error parsing event E3Requested"); // TODO: provide more info
                return None;
            };
            Some(EnclaveEvent::from(E3RequestedWithChainId(event, chain_id)))
        }
        Some(&IEnclave::CiphertextOutputPublished::SIGNATURE_HASH) => {
            let Ok(event) = IEnclave::CiphertextOutputPublished::decode_log_data(data, true) else {
                println!("Error parsing event CiphertextOutputPublished"); // TODO: provide more info
                return None;
            };
            Some(EnclaveEvent::from(event))
        }

        _ => {
            println!("Unknown event");
            return None;
        }
    }
}

/// Connects to Enclave.sol converting EVM events to EnclaveEvents
pub struct EnclaveSolReader {
    provider: ReadonlyProvider,
    contract_address: Address,
    bus: Recipient<EnclaveEvent>,
}

impl EnclaveSolReader {
    pub async fn new(
        bus: Addr<EventBus>,
        contract_address: Address,
        rpc_url: &str,
    ) -> Result<Self> {
        let provider = create_readonly_provider(rpc_url).await?;
        Ok(Self {
            contract_address,
            provider,
            bus: bus.into(),
        })
    }

    pub async fn attach(
        bus: Addr<EventBus>,
        rpc_url: &str,
        contract_address: &str,
    ) -> Result<Addr<Self>> {
        let addr = EnclaveSolReader::new(bus.clone(), contract_address.parse()?, rpc_url)
            .await?
            .start();

        println!("Evm is listening to {}", contract_address);
        Ok(addr)
    }
}

impl Actor for EnclaveSolReader {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let bus = self.bus.clone();
        let provider = self.provider.clone();
        let filter = Filter::new()
            .address(self.contract_address)
            .from_block(BlockNumberOrTag::Latest);

        ctx.spawn(
            async move { helpers::stream_from_evm(provider, filter, bus, extractor).await }
                .into_actor(self),
        );
    }
}
