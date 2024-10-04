use actix::prelude::*;
use actix::{Addr, Recipient};
use alloy::primitives::{LogData, B256};
use alloy::{
    eips::BlockNumberOrTag,
    primitives::Address,
    providers::{ProviderBuilder, RootProvider},
    rpc::types::Filter,
    sol,
    sol_types::SolEvent,
    transports::BoxTransport,
};
use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::{Bytes, U256},
    providers::{
        fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller},
        Identity,
    },
    rpc::types::TransactionReceipt,
    signers::local::PrivateKeySigner,
};
use anyhow::Result;
use enclave_core::{EnclaveErrorType, FromError, PlaintextAggregated, Subscribe};
use enclave_core::{EnclaveEvent, EventBus};
use std::sync::Arc;

use crate::helpers;

type WriterContractProvider = FillProvider<
    JoinFill<
        JoinFill<JoinFill<JoinFill<Identity, GasFiller>, NonceFiller>, ChainIdFiller>,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider<BoxTransport>,
    BoxTransport,
    Ethereum,
>;

sol! {
    #[derive(Debug)]
    struct E3 {
        uint256 seed;
        uint32[2] threshold;
        uint256[2] startWindow;
        uint256 duration;
        uint256 expiration;
        bytes32 encryptionSchemeId;
        address e3Program;
        bytes e3ProgramParams;
        address inputValidator;
        address decryptionVerifier;
        bytes32 committeePublicKey;
        bytes32 ciphertextOutput;
        bytes plaintextOutput;
    }

    #[derive(Debug)]
    event CiphertextOutputPublished(
        uint256 indexed e3Id,
        bytes ciphertextOutput
    );

    #[derive(Debug)]
    event E3Requested(
        uint256 e3Id,
        E3 e3,
        address filter,
        address indexed e3Program
    );

    #[derive(Debug)]
    #[sol(rpc)]
    contract Enclave {
        function publishPlaintextOutput(uint256 e3Id, bytes memory plaintextOutput, bytes memory proof) external returns (bool success);
    }
}

impl From<E3Requested> for enclave_core::E3Requested {
    fn from(value: E3Requested) -> Self {
        enclave_core::E3Requested {
            params: value.e3.e3ProgramParams.to_vec(),
            threshold_m: value.e3.threshold[0] as usize,
            seed: value.e3.seed.into(),
            e3_id: value.e3Id.to_string().into(),
        }
    }
}

impl From<E3Requested> for EnclaveEvent {
    fn from(value: E3Requested) -> Self {
        let payload: enclave_core::E3Requested = value.into();
        EnclaveEvent::from(payload)
    }
}

impl From<CiphertextOutputPublished> for enclave_core::CiphertextOutputPublished {
    fn from(value: CiphertextOutputPublished) -> Self {
        enclave_core::CiphertextOutputPublished {
            e3_id: value.e3Id.to_string().into(),
            ciphertext_output: value.ciphertextOutput.to_vec(),
        }
    }
}

impl From<CiphertextOutputPublished> for EnclaveEvent {
    fn from(value: CiphertextOutputPublished) -> Self {
        let payload: enclave_core::CiphertextOutputPublished = value.into();
        EnclaveEvent::from(payload)
    }
}

pub struct EnclaveSolReader {
    provider: Arc<RootProvider<BoxTransport>>,
    filter: Filter,
    bus: Recipient<EnclaveEvent>,
}

fn extractor(data: &LogData, topic: Option<&B256>) -> Option<EnclaveEvent> {
    match topic {
        Some(&E3Requested::SIGNATURE_HASH) => {
            let Ok(event) = E3Requested::decode_log_data(data, true) else {
                println!("Error parsing event E3Requested"); // TODO: provide more info
                return None;
            };
            Some(EnclaveEvent::from(event))
        }
        Some(&CiphertextOutputPublished::SIGNATURE_HASH) => {
            let Ok(event) = CiphertextOutputPublished::decode_log_data(data, true) else {
                println!("Error parsing event CiphertextOutputPublished"); // TODO: provide more info
                return None;
            };
            Some(EnclaveEvent::from(event))
        }

        _ => {
            println!("Unknown event");
            return None;
        }
    }
}

impl Actor for EnclaveSolReader {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let bus = self.bus.clone();
        let provider = self.provider.clone();
        let filter = self.filter.clone();
        ctx.spawn(
            async move { helpers::stream_from_evm(provider, filter, bus, extractor).await }
                .into_actor(self),
        );
    }
}

impl EnclaveSolReader {
    pub async fn new(
        bus: Addr<EventBus>,
        contract_address: Address,
        rpc_url: &str,
    ) -> Result<Self> {
        let filter = Filter::new()
            .address(contract_address)
            .from_block(BlockNumberOrTag::Latest);

        let provider: Arc<RootProvider<BoxTransport>> =
            Arc::new(ProviderBuilder::new().on_builtin(rpc_url).await?.into());

        Ok(Self {
            filter,
            provider,
            bus: bus.into(),
        })
    }

    pub async fn attach(
        bus: Addr<EventBus>,
        rpc_url: &str,
        contract_address: Address,
    ) -> Result<Addr<Self>> {
        let addr = EnclaveSolReader::new(bus.clone(), contract_address, rpc_url)
            .await?
            .start();

        println!("Evm is listening to {}", contract_address);
        Ok(addr)
    }
}

pub struct EnclaveSolWriter {
    provider: Arc<WriterContractProvider>,
    contract_address: Address,
    bus: Addr<EventBus>,
}

impl EnclaveSolWriter {
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
    ) -> Result<Addr<EnclaveSolWriter>> {
        let addr = EnclaveSolWriter::new(bus.clone(), rpc_url, contract_address)
            .await?
            .start();
        let _ = bus
            .send(Subscribe::new("PlaintextAggregated", addr.clone().into()))
            .await;

        Ok(addr)
    }
}

impl Actor for EnclaveSolWriter {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for EnclaveSolWriter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::PlaintextAggregated { data, .. } => ctx.notify(data),
            _ => (),
        }
    }
}

impl Handler<PlaintextAggregated> for EnclaveSolWriter {
    type Result = ResponseFuture<()>;
    fn handle(&mut self, msg: PlaintextAggregated, _: &mut Self::Context) -> Self::Result {
        let e3_id: U256 = msg.e3_id.try_into().unwrap();
        let decrypted_output = Bytes::from(msg.decrypted_output);
        let proof = Bytes::from(vec![1]);
        let contract_address = self.contract_address.clone();
        let provider = self.provider.clone();
        let bus = self.bus.clone();
        Box::pin(async move {
            match publish_plaintext_output(
                provider,
                contract_address,
                e3_id,
                decrypted_output,
                proof,
            )
            .await
            {
                Ok(_) => {
                    // log val
                }
                Err(err) => bus.do_send(EnclaveEvent::from_error(EnclaveErrorType::Evm, err)),
            }
        })
    }
}

async fn publish_plaintext_output(
    provider: Arc<WriterContractProvider>,
    contract_address: Address,
    e3_id: U256,
    decrypted_output: Bytes,
    proof: Bytes,
) -> Result<TransactionReceipt> {
    let contract = Enclave::new(contract_address, &provider);
    let builder = contract.publishPlaintextOutput(e3_id, decrypted_output, proof);
    let receipt = builder.send().await?.get_receipt().await?;
    Ok(receipt)
}
