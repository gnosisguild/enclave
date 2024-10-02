use actix::prelude::*;
use alloy::primitives::{Address, Bytes, U256};
use std::collections::HashMap;
use std::sync::Arc;

use crate::enclave_core::{EnclaveEvent, EventBus, Subscribe};
use crate::sortition::{GetNodes, Sortition};

use super::EVMContract;


pub struct EvmCaller {
    contracts: HashMap<String, Arc<EVMContract>>,
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
}

impl Actor for EvmCaller {
    type Context = Context<Self>;
}

impl EvmCaller {
    pub fn new(
        bus: Addr<EventBus>,
        sortition: Addr<Sortition>,
    ) -> Self {
        Self {
            contracts: HashMap::new(),
            bus,
            sortition,
        }
    }

    pub fn add_contract(&mut self, name: &str, contract: Arc<EVMContract>) {
        self.contracts.insert(name.to_string(), contract);
    }

    pub fn attach(
        bus: Addr<EventBus>,
        sortition: Addr<Sortition>,
    ) -> Addr<Self> {
        let addr = Self::new(bus.clone(), sortition).start();

        bus.do_send(Subscribe::new(
            "PublicKeyAggregated",
            addr.clone().recipient(),
        ));

        bus.do_send(Subscribe::new(
            "PlaintextAggregated",
            addr.clone().recipient(),
        ));

        addr
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddContract {
    pub name: String,
    pub contract: Arc<EVMContract>,
}

impl Handler<AddContract> for EvmCaller {
    type Result = ();

    fn handle(&mut self, msg: AddContract, _: &mut Self::Context) -> Self::Result {
        self.add_contract(&msg.name, msg.contract);
    }
}

impl Handler<EnclaveEvent> for EvmCaller {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        let contracts = self.contracts.clone();
        let sortition = self.sortition.clone();

        Box::pin(
            async move {
                match msg {
                    EnclaveEvent::PublicKeyAggregated { data, .. } => {
                        if let Some(contract) = contracts.get("registry") {
                            let nodes = sortition.send(GetNodes).await.unwrap_or_default();
                            let nodes: Vec<Address> = nodes
                                .into_iter()
                                .filter_map(|node| node.parse().ok())
                                .collect();

                            match contract
                                .publish_committee(
                                    U256::from_str_radix(&data.e3_id.0, 10).unwrap(),
                                    nodes,
                                    Bytes::from(data.pubkey),
                                )
                                .await 
                            {
                                Ok(tx) => println!("Published committee public key {:?}", tx.transaction_hash),
                                Err(e) => eprintln!("Failed to publish committee public key: {:?}", e),
                            }
                        }
                    }
                    EnclaveEvent::PlaintextAggregated { data, .. } => {
                        if let Some(contract) = contracts.get("enclave") {
                            println!("Publishing plaintext output {:?}", data.e3_id);
                            match contract
                                .publish_plaintext_output(
                                    U256::from_str_radix(&data.e3_id.0, 10).unwrap(),
                                    Bytes::from(data.decrypted_output),
                                    Bytes::from(vec![1]), // TODO: Implement proof generation
                                )
                                .await
                            {
                                Ok(tx) => println!("Published plaintext output {:?}", tx.transaction_hash),
                                Err(e) => eprintln!("Failed to publish plaintext: {:?}", e),
                            }
                        }
                    }
                    _ => {}
                }
            }
            .into_actor(self)
            .map(|_, _, _| ()),
        )
    }
}


pub async fn connect_evm_caller(
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
    rpc_url: &str,
    enclave_contract: Address,
    registry_contract: Address,
) -> Result<Addr<EvmCaller>, anyhow::Error> {
    let evm_caller = EvmCaller::attach(bus.clone(), sortition.clone());

    let enclave_instance = EVMContract::new(rpc_url, enclave_contract).await?;
    let registry_instance = EVMContract::new(rpc_url, registry_contract).await?;

    evm_caller.send(AddContract {
        name: "enclave".to_string(),
        contract: Arc::new(enclave_instance),
    }).await?;

    evm_caller.send(AddContract {
        name: "registry".to_string(),
        contract: Arc::new(registry_instance),
    }).await?;

    Ok(evm_caller)
}
