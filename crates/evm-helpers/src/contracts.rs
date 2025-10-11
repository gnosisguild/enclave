// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::providers::fillers::BlobGasFiller;
use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::{Address, Bytes, U256},
    providers::fillers::{
        ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
    },
    providers::{Identity, Provider, ProviderBuilder, RootProvider},
    rpc::types::TransactionReceipt,
    signers::local::PrivateKeySigner,
    sol,
};
use async_trait::async_trait;
use eyre::Result;
use once_cell::sync::Lazy;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::Mutex;

static NONCE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub async fn next_pending_nonce<P>(provider: &P) -> eyre::Result<u64>
where
    P: Provider<Ethereum> + Send + Sync,
{
    let from = provider.get_accounts().await?[0];
    provider
        .get_transaction_count(from)
        .pending()
        .await
        .map_err(Into::into)
}

sol! {
    #[derive(Debug)]
    struct E3 {
        uint256 seed;
        uint32[2] threshold;
        uint256 requestBlock;
        uint256[2] startWindow;
        uint256 duration;
        uint256 expiration;
        bytes32 encryptionSchemeId;
        address e3Program;
        bytes e3ProgramParams;
        bytes customParams;
        address inputValidator;
        address decryptionVerifier;
        bytes32 committeePublicKey;
        bytes32 ciphertextOutput;
        bytes plaintextOutput;
    }

    #[derive(Debug)]
    struct E3RequestParams {
        address filter;
        uint32[2] threshold;
        uint256[2] startWindow;
        uint256 duration;
        address e3Program;
        bytes e3ProgramParams;
        bytes computeProviderParams;
        bytes customParams;
    }

    #[derive(Debug)]
    #[sol(rpc)]
    contract Enclave {
        uint256 public nexte3Id = 0;
        mapping(uint256 e3Id => uint256 inputCount) public inputCounts;
        mapping(uint256 e3Id => bytes params) public e3Params;
        mapping(address e3Program => bool allowed) public e3Programs;
        function request(E3RequestParams memory request) external payable returns (uint256 e3Id, E3 memory e3);
        function activate(uint256 e3Id,bytes calldata publicKey) external returns (bool success);
        function enableE3Program(address e3Program) public onlyOwner returns (bool success);
        function publishInput(uint256 e3Id, bytes calldata data) external returns (bool success);
        function publishCiphertextOutput(uint256 e3Id, bytes calldata ciphertextOutput, bytes calldata proof) external returns (bool success);
        function publishPlaintextOutput(uint256 e3Id, bytes calldata data) external returns (bool success);
        function getE3(uint256 e3Id) external view returns (E3 memory e3);
        function getInputRoot(uint256 e3Id) public view returns (uint256);
        function getE3Quote(E3RequestParams memory request) external view returns (uint256 fee);
    }
}

/// Trait for read-only operations on the Enclave contract
#[async_trait]
pub trait EnclaveRead {
    /// Get the next E3 ID
    async fn get_e3_id(&self) -> Result<U256>;

    /// Get the details of an E3 by ID
    async fn get_e3(&self, e3_id: U256) -> Result<E3>;

    /// Get the input count for a specific E3 ID
    async fn get_input_count(&self, e3_id: U256) -> Result<U256>;

    /// Get the latest block number
    async fn get_latest_block(&self) -> Result<u64>;

    /// Get the root for a specific ID
    async fn get_input_root(&self, id: U256) -> Result<U256>;

    /// Get E3 parameters for a specific E3 ID
    async fn get_e3_params(&self, e3_id: U256) -> Result<Bytes>;

    /// Check if an E3 program is enabled
    async fn is_e3_program_enabled(&self, e3_program: Address) -> Result<bool>;

    /// Get the fee quote for an E3 request
    async fn get_e3_quote(
        &self,
        filter: Address,
        threshold: [u32; 2],
        start_window: [U256; 2],
        duration: U256,
        e3_program: Address,
        e3_params: Bytes,
        compute_provider_params: Bytes,
    ) -> Result<U256>;
}

/// Trait for write operations on the Enclave contract
#[async_trait]
pub trait EnclaveWrite {
    /// Request a new E3
    async fn request_e3(
        &self,
        filter: Address,
        threshold: [u32; 2],
        start_window: [U256; 2],
        duration: U256,
        e3_program: Address,
        e3_params: Bytes,
        compute_provider_params: Bytes,
        custom_params: Bytes,
    ) -> Result<TransactionReceipt>;

    /// Activate an E3 with a public key
    async fn activate(&self, e3_id: U256, pub_key: Bytes) -> Result<TransactionReceipt>;

    /// Enable an E3 program
    async fn enable_e3_program(&self, e3_program: Address) -> Result<TransactionReceipt>;

    /// Publish input data for an E3
    async fn publish_input(&self, e3_id: U256, data: Bytes) -> Result<TransactionReceipt>;

    /// Publish ciphertext output with proof
    async fn publish_ciphertext_output(
        &self,
        e3_id: U256,
        data: Bytes,
        proof: Bytes,
    ) -> Result<TransactionReceipt>;

    /// Publish plaintext output
    async fn publish_plaintext_output(
        &self,
        e3_id: U256,
        data: Bytes,
    ) -> Result<TransactionReceipt>;
}

/// Generic type to represent different provider types
pub trait ProviderType: Send {
    type Provider: Provider + Send + Sync + 'static;
}

/// Marker type for read-only provider
#[derive(Clone)]
pub struct ReadOnly;
impl ProviderType for ReadOnly {
    type Provider = EnclaveReadOnlyProvider;
}
/// Marker type for read-write provider
#[derive(Clone)]
pub struct ReadWrite;
impl ProviderType for ReadWrite {
    type Provider = EnclaveWriteProvider;
}

/// Generic Enclave contract
#[derive(Clone)]
pub struct EnclaveContract<T: ProviderType> {
    pub provider: Arc<T::Provider>,
    pub contract_address: Address,
    _marker: PhantomData<T>,
}

impl EnclaveContract<ReadWrite> {
    pub async fn new(
        http_rpc_url: &str,
        private_key: &str,
        contract_address: &str,
    ) -> Result<EnclaveContract<ReadWrite>> {
        EnclaveContractFactory::create_write(http_rpc_url, contract_address, private_key).await
    }

    pub fn get_provider(&self) -> Arc<EnclaveWriteProvider> {
        self.provider.clone()
    }

    pub fn address(&self) -> &Address {
        &self.contract_address
    }
}

impl EnclaveContract<ReadOnly> {
    pub async fn read_only(
        http_rpc_url: &str,
        contract_address: &str,
    ) -> Result<EnclaveContract<ReadOnly>> {
        EnclaveContractFactory::create_read(http_rpc_url, contract_address).await
    }

    pub fn get_provider(&self) -> Arc<EnclaveReadOnlyProvider> {
        self.provider.clone()
    }

    pub fn address(&self) -> &Address {
        &self.contract_address
    }
}

/// Type alias for read-only provider
pub type EnclaveReadOnlyProvider = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    RootProvider,
>;

/// Type alias for read-write provider
pub type EnclaveWriteProvider = FillProvider<
    JoinFill<
        JoinFill<
            JoinFill<
                Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            WalletFiller<EthereumWallet>,
        >,
        NonceFiller,
    >,
    RootProvider<Ethereum>,
    Ethereum,
>;

/// Type aliases for the two contract variants
pub type EnclaveReadContract = EnclaveContract<ReadOnly>;
pub type EnclaveWriteContract = EnclaveContract<ReadWrite>;

// Factory for creating contract instances
pub struct EnclaveContractFactory;

impl EnclaveContractFactory {
    /// Create a write-capable contract
    pub async fn create_write(
        http_rpc_url: &str,
        contract_address: &str,
        private_key: &str,
    ) -> Result<EnclaveContract<ReadWrite>> {
        let contract_address = contract_address.parse()?;

        let signer: PrivateKeySigner = private_key.parse()?;
        let wallet = EthereumWallet::from(signer);
        let provider = ProviderBuilder::new()
            .wallet(wallet)
            .with_cached_nonce_management()
            .connect(http_rpc_url)
            .await?;

        Ok(EnclaveContract::<ReadWrite> {
            provider: Arc::new(provider),
            contract_address,
            _marker: PhantomData,
        })
    }

    /// Create a read-only contract
    pub async fn create_read(
        http_rpc_url: &str,
        contract_address: &str,
    ) -> Result<EnclaveContract<ReadOnly>> {
        let contract_address = contract_address.parse()?;

        let provider = ProviderBuilder::new().connect(http_rpc_url).await?;

        Ok(EnclaveContract::<ReadOnly> {
            provider: Arc::new(provider),
            contract_address,
            _marker: PhantomData,
        })
    }
}

// Implement EnclaveRead for any EnclaveContract regardless of provider type
#[async_trait]
impl<T: Send + Sync> EnclaveRead for EnclaveContract<T>
where
    T: ProviderType,
{
    async fn get_e3_id(&self) -> Result<U256> {
        let contract = Enclave::new(self.contract_address, &self.provider);
        let e3_id = contract.nexte3Id().call().await?;
        Ok(e3_id)
    }

    async fn get_e3(&self, e3_id: U256) -> Result<E3> {
        let contract = Enclave::new(self.contract_address, &self.provider);
        let e3_return = contract.getE3(e3_id).call().await?;
        Ok(e3_return)
    }

    async fn get_input_count(&self, e3_id: U256) -> Result<U256> {
        let contract = Enclave::new(self.contract_address, &self.provider);
        let input_count = contract.inputCounts(e3_id).call().await?;
        Ok(input_count)
    }

    async fn get_latest_block(&self) -> Result<u64> {
        let block = self.provider.get_block_number().await?;
        Ok(block)
    }

    async fn get_input_root(&self, id: U256) -> Result<U256> {
        let contract = Enclave::new(self.contract_address, &self.provider);
        let root = contract.getInputRoot(id).call().await?;
        Ok(root)
    }

    async fn get_e3_params(&self, e3_id: U256) -> Result<Bytes> {
        let contract = Enclave::new(self.contract_address, &self.provider);
        let params = contract.e3Params(e3_id).call().await?;
        Ok(params)
    }

    async fn is_e3_program_enabled(&self, e3_program: Address) -> Result<bool> {
        let contract = Enclave::new(self.contract_address, &self.provider);
        let enabled = contract.e3Programs(e3_program).call().await?;
        Ok(enabled)
    }

    async fn get_e3_quote(
        &self,
        filter: Address,
        threshold: [u32; 2],
        start_window: [U256; 2],
        duration: U256,
        e3_program: Address,
        e3_params: Bytes,
        compute_provider_params: Bytes,
    ) -> Result<U256> {
        let e3_request = E3RequestParams {
            filter,
            threshold,
            startWindow: start_window,
            duration,
            e3Program: e3_program,
            e3ProgramParams: e3_params,
            computeProviderParams: compute_provider_params,
        };

        let contract = Enclave::new(self.contract_address, &self.provider);
        let fee = contract.getE3Quote(e3_request).call().await?;
        Ok(fee)
    }
}

// Implement EnclaveWrite only for contracts with ReadWrite marker
#[async_trait]
impl EnclaveWrite for EnclaveContract<ReadWrite> {
    async fn request_e3(
        &self,
        filter: Address,
        threshold: [u32; 2],
        start_window: [U256; 2],
        duration: U256,
        e3_program: Address,
        e3_params: Bytes,
        compute_provider_params: Bytes,
        custom_params: Bytes,
    ) -> Result<TransactionReceipt> {
        let _guard = NONCE_LOCK.lock().await;
        let nonce = next_pending_nonce(&*self.provider).await?;

        let e3_request = E3RequestParams {
            filter,
            threshold,
            startWindow: start_window,
            duration,
            e3Program: e3_program,
            e3ProgramParams: e3_params.clone(),
            computeProviderParams: compute_provider_params.clone(),
            customParams: custom_params.clone(),
        };

        let contract = Enclave::new(self.contract_address, &self.provider);
        let builder = contract
            .request(e3_request)
            .value(U256::from(1))
            .nonce(nonce);
        let receipt = builder.send().await?.get_receipt().await?;

        Ok(receipt)
    }

    async fn activate(&self, e3_id: U256, pub_key: Bytes) -> Result<TransactionReceipt> {
        let _guard = NONCE_LOCK.lock().await;
        let nonce = next_pending_nonce(&*self.provider).await?;

        let contract = Enclave::new(self.contract_address, &self.provider);
        let builder = contract.activate(e3_id, pub_key).nonce(nonce);
        let receipt = builder.send().await?.get_receipt().await?;

        Ok(receipt)
    }

    async fn enable_e3_program(&self, e3_program: Address) -> Result<TransactionReceipt> {
        let _guard = NONCE_LOCK.lock().await;
        let nonce = next_pending_nonce(&*self.provider).await?;

        let contract = Enclave::new(self.contract_address, &self.provider);
        let builder = contract.enableE3Program(e3_program).nonce(nonce);
        let receipt = builder.send().await?.get_receipt().await?;

        Ok(receipt)
    }

    async fn publish_input(&self, e3_id: U256, data: Bytes) -> Result<TransactionReceipt> {
        let _guard = NONCE_LOCK.lock().await;
        let nonce = next_pending_nonce(&*self.provider).await?;

        let contract = Enclave::new(self.contract_address, &self.provider);
        let builder = contract.publishInput(e3_id, data).nonce(nonce);
        let receipt = builder.send().await?.get_receipt().await?;

        Ok(receipt)
    }

    async fn publish_ciphertext_output(
        &self,
        e3_id: U256,
        data: Bytes,
        proof: Bytes,
    ) -> Result<TransactionReceipt> {
        let _guard = NONCE_LOCK.lock().await;
        let nonce = next_pending_nonce(&*self.provider).await?;

        let contract = Enclave::new(self.contract_address, &self.provider);
        let builder = contract
            .publishCiphertextOutput(e3_id, data, proof)
            .nonce(nonce);
        let receipt = builder.send().await?.get_receipt().await?;

        Ok(receipt)
    }

    async fn publish_plaintext_output(
        &self,
        e3_id: U256,
        data: Bytes,
    ) -> Result<TransactionReceipt> {
        let _guard = NONCE_LOCK.lock().await;
        let nonce = next_pending_nonce(&*self.provider).await?;

        let contract = Enclave::new(self.contract_address, &self.provider);
        let builder = contract.publishPlaintextOutput(e3_id, data).nonce(nonce);
        let receipt = builder.send().await?.get_receipt().await?;

        Ok(receipt)
    }
}
