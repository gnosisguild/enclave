use crate::helpers::{RpcWSClient, SignerProvider, WithChainId};
use actix::prelude::*;
use alloy::{
    primitives::{Address, Bytes, U256},
    rpc::types::TransactionReceipt,
    sol,
};
use anyhow::Result;
use events::{
    BusError, E3id, EnclaveErrorType, EnclaveEvent, EventBus, OrderedSet, PublicKeyAggregated,
    Shutdown, Subscribe,
};
use tracing::info;

sol!(
    #[sol(rpc)]
    NaiveRegistryFilter,
    "../../evm/artifacts/contracts/registry/NaiveRegistryFilter.sol/NaiveRegistryFilter.json"
);

pub struct RegistryFilterSolWriter {
    provider: WithChainId<SignerProvider<RpcWSClient>, RpcWSClient>,
    contract_address: Address,
    bus: Addr<EventBus>,
}

impl RegistryFilterSolWriter {
    pub async fn new(
        bus: &Addr<EventBus>,
        provider: &WithChainId<SignerProvider<RpcWSClient>, RpcWSClient>,
        contract_address: Address,
    ) -> Result<Self> {
        Ok(Self {
            provider: provider.clone(),
            contract_address,
            bus: bus.clone(),
        })
    }

    pub async fn attach(
        bus: &Addr<EventBus>,
        provider: &WithChainId<SignerProvider<RpcWSClient>, RpcWSClient>,
        contract_address: &str,
    ) -> Result<Addr<RegistryFilterSolWriter>> {
        let addr = RegistryFilterSolWriter::new(bus, provider, contract_address.parse()?)
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
            EnclaveEvent::PublicKeyAggregated { data, .. } => {
                // Only publish if the src and destination chains match
                if self.provider.get_chain_id() == data.src_chain_id {
                    ctx.notify(data);
                }
            }
            EnclaveEvent::Shutdown { data, .. } => ctx.notify(data),
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
                        info!(tx=%receipt.transaction_hash,"tx");
                    }
                    Err(err) => bus.err(EnclaveErrorType::Evm, err),
                }
            }
        })
    }
}

impl Handler<Shutdown> for RegistryFilterSolWriter {
    type Result = ();
    fn handle(&mut self, _: Shutdown, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

pub async fn publish_committee(
    provider: WithChainId<SignerProvider<RpcWSClient>, RpcWSClient>,
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
    let contract = NaiveRegistryFilter::new(contract_address, provider.get_provider());
    let builder = contract.publishCommittee(e3_id, nodes, public_key);
    let receipt = builder.send().await?.get_receipt().await?;
    Ok(receipt)
}

pub struct RegistryFilterSol;
impl RegistryFilterSol {
    pub async fn attach(
        bus: &Addr<EventBus>,
        provider: &WithChainId<SignerProvider<RpcWSClient>, RpcWSClient>,
        contract_address: &str,
    ) -> Result<()> {
        RegistryFilterSolWriter::attach(bus, provider, contract_address).await?;
        Ok(())
    }
}
