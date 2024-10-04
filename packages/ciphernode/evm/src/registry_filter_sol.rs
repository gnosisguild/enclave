use actix::prelude::*;
use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::{Address, Bytes, U256},
    providers::{
        fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller},
        Identity, ProviderBuilder, RootProvider,
    },
    rpc::types::TransactionReceipt,
    signers::local::PrivateKeySigner,
    sol,
    transports::BoxTransport,
};
use anyhow::Result;
use enclave_core::{
    EnclaveErrorType, EnclaveEvent, EventBus, FromError, PublicKeyAggregated, Subscribe,
};
use std::env;
use std::sync::Arc;

sol! {
    #[derive(Debug)]
    #[sol(rpc)]
    contract RegistryFilter {
        function publishCommittee(uint256 e3Id, address[] memory nodes, bytes memory publicKey) external onlyOwner;
    }
}

type ContractProvider = FillProvider<
    JoinFill<
        JoinFill<JoinFill<JoinFill<Identity, GasFiller>, NonceFiller>, ChainIdFiller>,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider<BoxTransport>,
    BoxTransport,
    Ethereum,
>;

pub struct RegistryFilterSolWriter {
    provider: Arc<ContractProvider>,
    contract_address: Address,
    bus: Addr<EventBus>,
}

impl RegistryFilterSolWriter {
    pub async fn new(
        bus: Addr<EventBus>,
        rpc_url: &str,
        contract_address: Address,
    ) -> Result<Self> {
        let signer: PrivateKeySigner = env::var("PRIVATE_KEY")?.parse()?;
        let wallet = EthereumWallet::from(signer.clone());
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_builtin(rpc_url)
            .await?;

        Ok(Self {
            provider: Arc::new(provider),
            contract_address,
            bus,
        })
    }

    pub async fn attach(
        bus: Addr<EventBus>,
        rpc_url: &str,
        contract_address: Address,
    ) -> Result<Addr<RegistryFilterSolWriter>> {
        let addr = RegistryFilterSolWriter::new(bus.clone(), rpc_url, contract_address)
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
        let e3_id: U256 = msg.e3_id.try_into().unwrap();
        let proof = Bytes::from(vec![1]);
        let contract_address = self.contract_address.clone();
        let provider = self.provider.clone();
        let bus = self.bus.clone();
        let nodes: Vec<Address> = msg
            .nodes
            .into_iter()
            .filter_map(|node| node.parse().ok())
            .collect();

        Box::pin(async move {
            match publish_committee(provider, contract_address, e3_id, nodes, proof).await {
                Ok(_) => {
                    // log val
                }
                Err(err) => bus.do_send(EnclaveEvent::from_error(EnclaveErrorType::Evm, err)),
            }
        })
    }
}

pub async fn publish_committee(
    provider: Arc<ContractProvider>,
    contract_address: Address,
    e3_id: U256,
    nodes: Vec<Address>,
    public_key: Bytes,
) -> Result<TransactionReceipt> {
    let contract = RegistryFilter::new(contract_address, &provider);
    let builder = contract.publishCommittee(e3_id, nodes, public_key);
    let receipt = builder.send().await?.get_receipt().await?;
    Ok(receipt)
}
