use super::ContractEvent;
use crate::{
    enclave_core::{self, EnclaveEvent, EventBus},
    evm::{AddEventHandler, AddListener, EvmContractManager, StartListening},
};
use actix::Addr;
use alloy::{primitives::Address, sol};
use anyhow::Result;

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

impl ContractEvent for CiphernodeAdded {
    fn process(&self, bus: Addr<EventBus>) -> Result<()> {
        let data: enclave_core::CiphernodeAdded = self.clone().into();
        bus.do_send(EnclaveEvent::from(data));
        Ok(())
    }
}

impl ContractEvent for CiphernodeRemoved {
    fn process(&self, bus: Addr<EventBus>) -> Result<()> {
        let data: enclave_core::CiphernodeRemoved = self.clone().into();
        bus.do_send(EnclaveEvent::from(data));
        Ok(())
    }
}

pub async fn connect_evm_ciphernode_registry(
    bus: Addr<EventBus>,
    rpc_url: &str,
    contract_address: Address,
) -> Result<()> {
    let evm_manager = EvmContractManager::attach(bus.clone(), rpc_url).await;
    let evm_listener = evm_manager.send(AddListener { contract_address }).await?;

    evm_listener
        .send(AddEventHandler::<CiphernodeAdded>::new())
        .await?;

    evm_listener
        .send(AddEventHandler::<CiphernodeRemoved>::new())
        .await?;

    evm_listener.do_send(StartListening);

    println!("Evm is listening to {}", contract_address);
    Ok(())
}
