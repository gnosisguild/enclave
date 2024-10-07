use crate::helpers::{create_provider_with_signer, Signer};
use actix::prelude::*;
use alloy::{
    primitives::{Address, Bytes, U256},
    rpc::types::TransactionReceipt,
    signers::local::PrivateKeySigner,
    sol,
};
use anyhow::Result;
use enclave_core::{
    BusError, E3id, EnclaveErrorType, EnclaveEvent, EventBus, OrderedSet, PublicKeyAggregated,
    Subscribe,
};
use std::{env, sync::Arc};

sol! {
    #[derive(Debug)]
    #[sol(rpc)]
    contract RegistryFilter {
        function publishCommittee(uint256 e3Id, address[] memory nodes, bytes memory publicKey) external onlyOwner;
    }
}

pub struct RegistryFilterSolWriter {
    provider: Signer,
    contract_address: Address,
    bus: Addr<EventBus>,
}

impl RegistryFilterSolWriter {
    pub async fn new(
        bus: Addr<EventBus>,
        rpc_url: &str,
        contract_address: Address,
        signer: Arc<PrivateKeySigner>,
    ) -> Result<Self> {
        Ok(Self {
            provider: create_provider_with_signer(rpc_url, signer).await?,
            contract_address,
            bus,
        })
    }

    pub async fn attach(
        bus: Addr<EventBus>,
        rpc_url: &str,
        contract_address: &str,
        signer: Arc<PrivateKeySigner>,
    ) -> Result<Addr<RegistryFilterSolWriter>> {
        let addr =
            RegistryFilterSolWriter::new(bus.clone(), rpc_url, contract_address.parse()?, signer)
                .await?
                .start();
        let _ = bus
            .send(Subscribe::new("PublicKeyAggregated", addr.clone().into()))
            .await;

        Ok(addr)
    }
}

impl Actor for RegistryFilterSolWriter {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for RegistryFilterSolWriter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::PublicKeyAggregated { data, .. } => ctx.notify(data),
            _ => (),
        }
    }
}

impl Handler<PublicKeyAggregated> for RegistryFilterSolWriter {
    type Result = ResponseFuture<()>;
    fn handle(&mut self, msg: PublicKeyAggregated, _: &mut Self::Context) -> Self::Result {
        Box::pin({
            let e3_id = msg.e3_id.clone();
            let pubkey = msg.pubkey.clone();
            let contract_address = self.contract_address.clone();
            let provider = self.provider.clone();
            let bus = self.bus.clone();
            let nodes = msg.nodes.clone();

            async move {
                let result =
                    publish_committee(provider, contract_address, e3_id, nodes, pubkey).await;
                match result {
                    Ok(receipt) => {
                        println!("tx:{}", receipt.transaction_hash);
                    }
                    Err(err) => bus.err(EnclaveErrorType::Evm, err),
                }
            }
        })
    }
}

pub async fn publish_committee(
    provider: Signer,
    contract_address: Address,
    e3_id: E3id,
    nodes: OrderedSet<String>,
    public_key: Vec<u8>,
) -> Result<TransactionReceipt> {
    let e3_id: U256 = e3_id.try_into()?;
    let public_key = Bytes::from(public_key);
    let nodes: Vec<Address> = nodes
        .into_iter()
        .filter_map(|node| node.parse().ok())
        .collect();
    let contract = RegistryFilter::new(contract_address, &provider);
    let builder = contract.publishCommittee(e3_id, nodes, public_key);
    let receipt = builder.send().await?.get_receipt().await?;
    Ok(receipt)
}

pub struct RegistryFilterSol;
impl RegistryFilterSol {
    pub async fn attach(
        bus: Addr<EventBus>,
        rpc_url: &str,
        contract_address: &str,
        signer: Arc<PrivateKeySigner>,
    ) -> Result<()> {
        RegistryFilterSolWriter::attach(bus.clone(), rpc_url, contract_address, signer).await?;
        Ok(())
    }
}
