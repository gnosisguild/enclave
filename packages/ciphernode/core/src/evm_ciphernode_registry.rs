use crate::{
    events,
    evm_listener::{AddEventHandler, ContractEvent, StartListening},
    evm_manager::{AddListener, EvmContractManager},
    EnclaveEvent, EventBus,
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

impl From<CiphernodeAdded> for events::CiphernodeAdded {
    fn from(value: CiphernodeAdded) -> Self {
        events::CiphernodeAdded {
            address: value.node.to_string(),
            index: value.index.as_limbs()[0] as usize,
            num_nodes: value.numNodes.as_limbs()[0] as usize,
        }
    }
}

impl From<CiphernodeRemoved> for events::CiphernodeRemoved {
    fn from(value: CiphernodeRemoved) -> Self {
        events::CiphernodeRemoved {
            address: value.node.to_string(),
            index: value.index.as_limbs()[0] as usize,
            num_nodes: value.numNodes.as_limbs()[0] as usize,
        }
    }
}

impl ContractEvent for CiphernodeAdded {
    fn process(&self, bus: Addr<EventBus>) -> Result<()> {
        println!("Ciphernode Added: {:?}", self);
        let data: events::CiphernodeAdded = self.clone().into();
        bus.do_send(EnclaveEvent::from(data));
        Ok(())
    }
}

impl ContractEvent for CiphernodeRemoved {
    fn process(&self, bus: Addr<EventBus>) -> Result<()> {
        let data: events::CiphernodeRemoved = self.clone().into();
        bus.do_send(EnclaveEvent::from(data));
        Ok(())
    }
}

pub async fn connect_evm_ciphernode_registry(
    bus: Addr<EventBus>,
    rpc_url: &str,
    contract_address: Address,
) {
    let evm_manager = EvmContractManager::attach(bus.clone(), rpc_url).await;
    let evm_listener = evm_manager
        .send(AddListener { contract_address })
        .await
        .unwrap();

    evm_listener
        .send(AddEventHandler::<CiphernodeAdded>::new())
        .await
        .unwrap();

    evm_listener
        .send(AddEventHandler::<CiphernodeRemoved>::new())
        .await
        .unwrap();

    evm_listener.do_send(StartListening);

    println!("Evm is listening.......");
}
