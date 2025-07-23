// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::helpers::EthProvider;
use actix::prelude::*;
use alloy::{
    primitives::{Address, Bytes, U256},
    providers::{Provider, WalletProvider},
    rpc::types::TransactionReceipt,
    sol,
};
use anyhow::Result;
use e3_events::{
    BusError, E3id, EnclaveErrorType, EnclaveEvent, EventBus, OrderedSet, PublicKeyAggregated,
    Shutdown, Subscribe,
};
use tracing::info;

sol!(
    #[sol(rpc)]
    NaiveRegistryFilter,
    "../../packages/evm/artifacts/contracts/registry/NaiveRegistryFilter.sol/NaiveRegistryFilter.json"
);

pub struct RegistryFilterSolWriter<P> {
    provider: EthProvider<P>,
    contract_address: Address,
    bus: Addr<EventBus<EnclaveEvent>>,
}

impl<P: Provider + WalletProvider + Clone + 'static> RegistryFilterSolWriter<P> {
    pub async fn new(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: EthProvider<P>,
        contract_address: Address,
    ) -> Result<Self> {
        Ok(Self {
            provider,
            contract_address,
            bus: bus.clone(),
        })
    }

    pub async fn attach(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: EthProvider<P>,
        contract_address: &str,
    ) -> Result<Addr<RegistryFilterSolWriter<P>>> {
        let addr = RegistryFilterSolWriter::new(bus, provider, contract_address.parse()?)
            .await?
            .start();

        let _ = bus
            .send(Subscribe::new("PublicKeyAggregated", addr.clone().into()))
            .await;

        Ok(addr)
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Actor for RegistryFilterSolWriter<P> {
    type Context = actix::Context<Self>;
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<EnclaveEvent>
    for RegistryFilterSolWriter<P>
{
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::PublicKeyAggregated { data, .. } => {
                // Only publish if the src and destination chains match
                if self.provider.chain_id() == data.e3_id.chain_id() {
                    ctx.notify(data);
                }
            }
            EnclaveEvent::Shutdown { data, .. } => ctx.notify(data),
            _ => (),
        }
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<PublicKeyAggregated>
    for RegistryFilterSolWriter<P>
{
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: PublicKeyAggregated, _: &mut Self::Context) -> Self::Result {
        Box::pin({
            let e3_id = msg.e3_id.clone();
            let pubkey = msg.pubkey.clone();
            let contract_address = self.contract_address;
            let provider = self.provider.clone();
            let bus = self.bus.clone();
            let nodes = msg.nodes.clone();

            async move {
                let result =
                    publish_committee(provider, contract_address, e3_id, nodes, pubkey).await;
                match result {
                    Ok(receipt) => {
                        info!(tx=%receipt.transaction_hash, "Transaction published");
                    }
                    Err(err) => bus.err(EnclaveErrorType::Evm, err),
                }
            }
        })
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<Shutdown>
    for RegistryFilterSolWriter<P>
{
    type Result = ();

    fn handle(&mut self, _: Shutdown, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

pub async fn publish_committee<P: Provider + WalletProvider + Clone>(
    provider: EthProvider<P>,
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
    let from_address = provider.provider().default_signer_address();
    let current_nonce = provider
        .provider()
        .get_transaction_count(from_address)
        .pending()
        .await?;
    let contract = NaiveRegistryFilter::new(contract_address, provider.provider());
    let builder = contract
        .publishCommittee(e3_id, nodes, public_key)
        .nonce(current_nonce);
    let receipt = builder.send().await?.get_receipt().await?;
    Ok(receipt)
}

pub struct RegistryFilterSol;

impl RegistryFilterSol {
    pub async fn attach<P: Provider + WalletProvider + Clone + 'static>(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: EthProvider<P>,
        contract_address: &str,
    ) -> Result<()> {
        RegistryFilterSolWriter::attach(bus, provider, contract_address).await?;
        Ok(())
    }
}
