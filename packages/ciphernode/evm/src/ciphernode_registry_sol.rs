use actix::{Actor, Addr, AsyncContext, Recipient, WrapFuture};
use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, LogData, B256},
    rpc::types::Filter,
    sol,
    sol_types::SolEvent,
};
use anyhow::Result;
use enclave_core::{EnclaveEvent, EventBus};

use crate::helpers::{self, create_readonly_provider, ReadonlyProvider};

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

/// Connects to CiphernodeRegistry.sol converting EVM events to EnclaveEvents
pub struct CiphernodeRegistrySolReader {
    provider: ReadonlyProvider,
    contract_address: Address,
    bus: Recipient<EnclaveEvent>,
}

impl CiphernodeRegistrySolReader {
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
        let addr =
            CiphernodeRegistrySolReader::new(bus.clone(), contract_address.parse()?, rpc_url)
                .await?
                .start();

        println!("CiphernodeRegistrySol is listening to {}", contract_address);
        Ok(addr)
    }
}

impl Actor for CiphernodeRegistrySolReader {
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

pub struct CiphernodeRegistrySol;
impl CiphernodeRegistrySol {
    pub async fn attach(bus: Addr<EventBus>, rpc_url: &str, contract_address: &str) -> Result<()> {
        CiphernodeRegistrySolReader::attach(bus.clone(), rpc_url, contract_address).await?;
        Ok(())
    }
}
