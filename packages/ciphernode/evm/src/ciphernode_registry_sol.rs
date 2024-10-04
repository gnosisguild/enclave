use actix::{Actor, Addr, AsyncContext, Recipient, WrapFuture};
use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, LogData, B256},
    providers::{ProviderBuilder, RootProvider},
    rpc::types::Filter,
    sol,
    sol_types::SolEvent,
    transports::BoxTransport,
};
use anyhow::Result;
use enclave_core::{EnclaveEvent, EventBus};
use std::sync::Arc;

use crate::helpers;

sol! {
    #[derive(Debug)]
    event CiphernodeAdded(
        address indexed node,
        uint256 index,
        uint256 numNodes,
        uint256 size
    );

    #[derive(Debug)]
    event CiphernodeRemoved(
        address indexed node,
        uint256 index,
        uint256 numNodes,
        uint256 size
    );
}

type ContractProvider = RootProvider<BoxTransport>;

impl From<CiphernodeAdded> for enclave_core::CiphernodeAdded {
    fn from(value: CiphernodeAdded) -> Self {
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

impl From<CiphernodeAdded> for EnclaveEvent {
    fn from(value: CiphernodeAdded) -> Self {
        let payload: enclave_core::CiphernodeAdded = value.into();
        EnclaveEvent::from(payload)
    }
}

impl From<CiphernodeRemoved> for enclave_core::CiphernodeRemoved {
    fn from(value: CiphernodeRemoved) -> Self {
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

impl From<CiphernodeRemoved> for EnclaveEvent {
    fn from(value: CiphernodeRemoved) -> Self {
        let payload: enclave_core::CiphernodeRemoved = value.into();
        EnclaveEvent::from(payload)
    }
}

pub struct CiphernodeRegistrySolReader {
    provider: Arc<ContractProvider>,
    bus: Recipient<EnclaveEvent>,
    filter: Filter,
}

fn extractor(data: &LogData, topic: Option<&B256>) -> Option<EnclaveEvent> {
    match topic {
        Some(&CiphernodeAdded::SIGNATURE_HASH) => {
            let Ok(event) = CiphernodeAdded::decode_log_data(data, true) else {
                println!("Error parsing event CiphernodeAdded"); // TODO: provide more info
                return None;
            };
            Some(EnclaveEvent::from(event))
        }
        Some(&CiphernodeRemoved::SIGNATURE_HASH) => {
            let Ok(event) = CiphernodeRemoved::decode_log_data(data, true) else {
                println!("Error parsing event CiphernodeRemoved"); // TODO: provide more info
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

impl CiphernodeRegistrySolReader {
    pub async fn new(
        bus: Addr<EventBus>,
        contract_address: Address,
        rpc_url: &str,
    ) -> Result<Self> {
        let filter = Filter::new()
            .address(contract_address)
            .from_block(BlockNumberOrTag::Latest);

        let provider: Arc<RootProvider<BoxTransport>> =
            Arc::new(ProviderBuilder::new().on_builtin(rpc_url).await?.into());

        Ok(Self {
            filter,
            provider,
            bus: bus.into(),
        })
    }

    pub async fn attach(
        bus: Addr<EventBus>,
        rpc_url: &str,
        contract_address: Address,
    ) -> Result<Addr<Self>> {
        let addr = CiphernodeRegistrySolReader::new(bus.clone(), contract_address, rpc_url)
            .await?
            .start();

        println!("Evm is listening to {}", contract_address);
        Ok(addr)
    }
}

impl Actor for CiphernodeRegistrySolReader {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let bus = self.bus.clone();
        let provider = self.provider.clone();
        let filter = self.filter.clone();
        ctx.spawn(
            async move { helpers::stream_from_evm(provider, filter, bus, extractor).await }
                .into_actor(self),
        );
    }
}
