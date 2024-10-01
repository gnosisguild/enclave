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
use std::env;
use std::sync::Arc;

sol! {
    #[derive(Debug)]
    #[sol(rpc)]
    contract Enclave {
        function publishPlaintextOutput(uint256 e3Id, bytes memory plaintextOutput, bytes memory proof) external returns (bool success);
    }

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

pub struct EVMContract {
    pub provider: Arc<ContractProvider>,
    pub contract_address: Address,
}

impl EVMContract {
    pub async fn new(rpc_url: &str, contract_address: Address) -> Result<Self> {
        let signer: PrivateKeySigner = env::var("PRIVATE_KEY")?.parse()?;
        let wallet = EthereumWallet::from(signer.clone());
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_builtin(rpc_url)
            .await?;

        Ok(Self {
            provider: Arc::new(provider),
            contract_address: contract_address,
        })
    }

    pub async fn publish_plaintext_output(
        &self,
        e3_id: U256,
        plaintext_output: Bytes,
        proof: Bytes,
    ) -> Result<TransactionReceipt> {
        let contract = Enclave::new(self.contract_address, &self.provider);
        let builder = contract.publishPlaintextOutput(e3_id, plaintext_output, proof);
        let receipt = builder.send().await?.get_receipt().await?;
        Ok(receipt)
    }

    pub async fn publish_committee(
        &self,
        e3_id: U256,
        nodes: Vec<Address>,
        public_key: Bytes,
    ) -> Result<TransactionReceipt> {
        let contract = RegistryFilter::new(self.contract_address, &self.provider);
        let builder = contract.publishCommittee(e3_id, nodes, public_key);
        let receipt = builder.send().await?.get_receipt().await?;
        Ok(receipt)
    }
}