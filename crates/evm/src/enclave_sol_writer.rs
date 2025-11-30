// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::helpers::EthProvider;
use actix::prelude::*;
use actix::Addr;
use alloy::{
    primitives::Address,
    providers::{Provider, WalletProvider},
    sol,
};
use alloy::{
    primitives::{Bytes, U256},
    rpc::types::TransactionReceipt,
};
use anyhow::Result;
use e3_events::prelude::*;
use e3_events::EnclaveEvent;
use e3_events::EnclaveEventData;
use e3_events::EventManager;
use e3_events::Shutdown;
use e3_events::{E3id, EnclaveErrorType, PlaintextAggregated, Subscribe};
use tracing::info;

sol!(
    #[sol(rpc)]
    IEnclave,
    "../../packages/enclave-contracts/artifacts/contracts/interfaces/IEnclave.sol/IEnclave.json"
);

/// Consumes events from the event bus and calls EVM methods on the Enclave.sol contract
pub struct EnclaveSolWriter<P> {
    provider: EthProvider<P>,
    contract_address: Address,
    bus: EventManager<EnclaveEvent>,
}

impl<P: Provider + WalletProvider + Clone + 'static> EnclaveSolWriter<P> {
    pub fn new(
        bus: &EventManager<EnclaveEvent>,
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
        bus: &EventManager<EnclaveEvent>,
        provider: EthProvider<P>,
        contract_address: &str,
    ) -> Result<Addr<EnclaveSolWriter<P>>> {
        let addr = EnclaveSolWriter::new(bus, provider, contract_address.parse()?)?.start();
        bus.subscribe_all(&["PlaintextAggregated", "Shutdown"], addr.clone().into());
        Ok(addr)
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Actor for EnclaveSolWriter<P> {
    type Context = actix::Context<Self>;
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<EnclaveEvent> for EnclaveSolWriter<P> {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg.into_data() {
            EnclaveEventData::PlaintextAggregated(data) => {
                // Only publish if the src and destination chains match
                if self.provider.chain_id() == data.e3_id.chain_id() {
                    ctx.notify(data);
                }
            }
            EnclaveEventData::Shutdown(data) => ctx.notify(data),
            _ => (),
        }
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<PlaintextAggregated>
    for EnclaveSolWriter<P>
{
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: PlaintextAggregated, _: &mut Self::Context) -> Self::Result {
        Box::pin({
            let e3_id = msg.e3_id.clone();
            let decrypted_output = msg.decrypted_output.clone();
            let contract_address = self.contract_address;
            let provider = self.provider.clone();
            let bus = self.bus.clone();
            async move {
                // HACK: plaintext format is now a Vec of ArcBytes for legacy tests for now we are extracting
                // the first entry and writing this will change once we make our legacy tests catch up
                let Some(decrypted) = decrypted_output.first() else {
                    bus.err(
                        EnclaveErrorType::Evm,
                        anyhow::anyhow!("Decrypted output was empty!"),
                    );
                    return;
                };
                let result = publish_plaintext_output(
                    provider,
                    contract_address,
                    e3_id,
                    decrypted.extract_bytes(),
                )
                .await;
                match result {
                    Ok(receipt) => {
                        info!(tx=%receipt.transaction_hash, "Published plaintext output");
                    }
                    Err(err) => {
                        bus.err(
                            EnclaveErrorType::Evm,
                            anyhow::anyhow!("Error publishing plaintext output: {:?}", err),
                        );
                    }
                }
            }
        })
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<Shutdown> for EnclaveSolWriter<P> {
    type Result = ();

    fn handle(&mut self, _: Shutdown, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

async fn publish_plaintext_output<P: Provider + WalletProvider + Clone>(
    provider: EthProvider<P>,
    contract_address: Address,
    e3_id: E3id,
    decrypted_output: Vec<u8>,
) -> Result<TransactionReceipt> {
    let e3_id: U256 = e3_id.try_into()?;
    let decrypted_output = Bytes::from(decrypted_output);
    let proof = Bytes::from(vec![1]);

    // Wait for ciphertext output transaction to propagate
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let from_address = provider.provider().default_signer_address();
    let current_nonce = provider
        .provider()
        .get_transaction_count(from_address)
        .pending()
        .await?;

    let contract = IEnclave::new(contract_address, provider.provider());
    info!("publishPlaintextOutput() e3_id={:?}", e3_id);
    let builder = contract
        .publishPlaintextOutput(e3_id, decrypted_output, proof)
        .nonce(current_nonce);
    let receipt = builder.send().await?.get_receipt().await?;
    Ok(receipt)
}
