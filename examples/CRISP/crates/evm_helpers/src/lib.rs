// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::{Address, U256},
    providers::{
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            WalletFiller,
        },
        Identity, ProviderBuilder, RootProvider,
    },
    rpc::types::TransactionReceipt,
    signers::local::PrivateKeySigner,
    sol,
};
use eyre::Result;
use std::sync::Arc;

sol! {
    #[derive(Debug)]
    #[sol(rpc)]
    contract CRISPProgram {
        function setRoundData(uint256 _root, address _token, uint256 _balanceThreshold) external;
    }
}

/// Type alias for write provider (same as EnclaveWriteProvider)
pub type CRISPWriteProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider<Ethereum>,
    Ethereum,
>;

/// CRISP contract instance for interacting with CRISPProgram
#[derive(Clone)]
pub struct CRISPContract {
    provider: Arc<CRISPWriteProvider>,
    contract_address: Address,
}

impl CRISPContract {
    /// Get the contract address
    pub fn address(&self) -> &Address {
        &self.contract_address
    }

    /// Create a new CRISP contract instance
    pub async fn new(
        http_rpc_url: &str,
        private_key: &str,
        contract_address: &str,
    ) -> Result<CRISPContract> {
        let contract_address = contract_address.parse()?;
        let signer: PrivateKeySigner = private_key.parse()?;
        let wallet = EthereumWallet::from(signer);
        let provider = ProviderBuilder::new()
            .wallet(wallet)
            .connect(http_rpc_url)
            .await?;

        Ok(CRISPContract {
            provider: Arc::new(provider),
            contract_address,
        })
    }

    /// Set round data on the CRISPProgram contract
    pub async fn set_round_data(
        &self,
        merkle_root: U256,
        token_address: Address,
        balance_threshold: U256,
    ) -> Result<TransactionReceipt> {
        let contract = CRISPProgram::new(self.contract_address, self.provider.as_ref());
        let receipt = contract
            .setRoundData(merkle_root, token_address, balance_threshold)
            .send()
            .await?
            .get_receipt()
            .await?;

        Ok(receipt)
    }
}

/// Factory for creating CRISP contract instances
pub struct CRISPContractFactory;

impl CRISPContractFactory {
    /// Create a write-capable contract
    pub async fn create_write(
        http_rpc_url: &str,
        contract_address: &str,
        private_key: &str,
    ) -> Result<CRISPContract> {
        CRISPContract::new(http_rpc_url, private_key, contract_address).await
    }
}
