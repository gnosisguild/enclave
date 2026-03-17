// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::error_decoder::format_evm_error;
use crate::helpers::EthProvider;
use crate::send_tx_with_retry;
use actix::prelude::*;
use alloy::{
    primitives::Address,
    providers::{Provider, WalletProvider},
    sol,
    sol_types::SolValue,
};
use alloy::{
    primitives::{Bytes, U256},
    rpc::types::TransactionReceipt,
};
use anyhow::Result;
use e3_events::BusHandle;
use e3_events::EnclaveEventData;
use e3_events::EventType;
use e3_events::Shutdown;
use e3_events::{prelude::*, EffectsEnabled};
use e3_events::{run_once, EnclaveEvent};
use e3_events::{E3Stage, E3StageChanged};
use e3_events::{E3id, EType, PlaintextAggregated, Proof};
use e3_utils::NotifySync;
use e3_utils::MAILBOX_LIMIT;
use tracing::info;

sol!(
    #[sol(rpc)]
    IEnclave,
    "../../packages/enclave-contracts/artifacts/contracts/interfaces/IEnclave.sol/IEnclave.json"
);

/// ABI-encodes a C7 proof for DecryptionVerifier.verify: abi.encode(rawProof, publicInputs).
/// Public input count is derived from proof data; the contract rejects invalid counts.
fn encode_c7_proof(proof: &Proof) -> Option<Bytes> {
    let signals: &[u8] = &*proof.public_signals;
    if signals.is_empty() || signals.len() % 32 != 0 {
        return None;
    }
    let count = signals.len() / 32;
    let mut inputs = Vec::with_capacity(count);
    for chunk in signals.chunks_exact(32) {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(chunk);
        inputs.push(arr);
    }
    let raw = Bytes::from((&*proof.data).to_vec());
    let encoded = (raw, inputs).abi_encode();
    Some(Bytes::from(encoded))
}

/// Consumes events from the event bus and calls EVM methods on the Enclave.sol contract
pub struct EnclaveSolWriter<P> {
    provider: EthProvider<P>,
    contract_address: Address,
    bus: BusHandle,
}

impl<P: Provider + WalletProvider + Clone + 'static> EnclaveSolWriter<P> {
    pub fn new(
        bus: &BusHandle,
        provider: EthProvider<P>,
        contract_address: Address,
    ) -> Result<Self> {
        Ok(Self {
            provider,
            contract_address,
            bus: bus.clone(),
        })
    }

    pub fn attach(bus: &BusHandle, provider: EthProvider<P>, contract_address: Address) {
        let addr = run_once::<EffectsEnabled>({
            let bus = bus.clone();
            move |_| {
                let addr = EnclaveSolWriter::new(&bus, provider, contract_address)?.start();
                bus.subscribe_all(
                    &[
                        EventType::PlaintextAggregated,
                        EventType::E3StageChanged,
                        EventType::Shutdown,
                    ],
                    addr.clone().into(),
                );
                Ok(())
            }
        });

        bus.subscribe(EventType::EffectsEnabled, addr.recipient());
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Actor for EnclaveSolWriter<P> {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT)
    }
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
            EnclaveEventData::E3StageChanged(data) => {
                // When an E3 transitions to Failed on-chain, call processE3Failure
                // to finalize refund distribution automatically.
                if data.new_stage == E3Stage::Failed
                    && self.provider.chain_id() == data.e3_id.chain_id()
                {
                    ctx.notify(data);
                }
            }
            EnclaveEventData::Shutdown(data) => self.notify_sync(ctx, data),
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
                    bus.err(EType::Evm, anyhow::anyhow!("Decrypted output was empty!"));
                    return;
                };
                // Reject multi-output results — partial on-chain write is worse than failing
                if decrypted_output.len() > 1 {
                    bus.err(
                        EType::Evm,
                        anyhow::anyhow!(
                            "E3 {} has {} decrypted outputs but only single-output is supported. \
                            Refusing partial on-chain write.",
                            e3_id,
                            decrypted_output.len()
                        ),
                    );
                    return;
                }
                if decrypted_output.len() != msg.aggregation_proofs.len() {
                    bus.err(
                        EType::Evm,
                        anyhow::anyhow!(
                            "E3 {} decrypted_output len ({}) != aggregation_proofs len ({})",
                            e3_id,
                            decrypted_output.len(),
                            msg.aggregation_proofs.len()
                        ),
                    );
                    return;
                }
                let result = publish_plaintext_output(
                    provider,
                    contract_address,
                    e3_id,
                    decrypted.extract_bytes(),
                    msg.aggregation_proofs.first(),
                )
                .await;
                match result {
                    Ok(receipt) => {
                        info!(tx=%receipt.transaction_hash, "Published plaintext output");
                    }
                    Err(err) => {
                        bus.err(
                            EType::Evm,
                            anyhow::anyhow!(
                                "Error publishing plaintext output: {}",
                                format_evm_error(&err)
                            ),
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

impl<P: Provider + WalletProvider + Clone + 'static> Handler<E3StageChanged>
    for EnclaveSolWriter<P>
{
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: E3StageChanged, _: &mut Self::Context) -> Self::Result {
        Box::pin({
            let contract_address = self.contract_address;
            let provider = self.provider.clone();
            async move {
                let result =
                    process_e3_failure(provider, contract_address, msg.e3_id.clone()).await;
                match result {
                    Ok(receipt) => {
                        info!(tx=%receipt.transaction_hash, "Called processE3Failure for E3 {}", msg.e3_id);
                    }
                    Err(err) => {
                        // Non-fatal: may revert if already processed or no payment
                        info!(
                            "processE3Failure for E3 {} did not succeed (may already be processed): {:?}",
                            msg.e3_id, err
                        );
                    }
                }
            }
        })
    }
}

async fn publish_plaintext_output<P: Provider + WalletProvider + Clone>(
    provider: EthProvider<P>,
    contract_address: Address,
    e3_id: E3id,
    decrypted_output: Vec<u8>,
    aggregation_proof: Option<&Proof>,
) -> Result<TransactionReceipt> {
    let e3_id: U256 = e3_id.try_into()?;

    let from_address = provider.provider().default_signer_address();
    let current_nonce = provider
        .provider()
        .get_transaction_count(from_address)
        .pending()
        .await?;

    let proof = aggregation_proof.and_then(encode_c7_proof).ok_or_else(|| {
        anyhow::anyhow!(
            "C7 proof missing or invalid (expected non-empty public_signals divisible by 32)"
        )
    })?;

    send_tx_with_retry(
        "publishPlaintextOutput",
        &["CiphertextOutputNotPublished"],
        || {
            info!("publishPlaintextOutput() e3_id={:?}", e3_id);
            let decrypted_output = Bytes::from(decrypted_output.clone());
            let proof = proof.clone();
            let contract = IEnclave::new(contract_address, provider.provider());

            async move {
                let builder = contract
                    .publishPlaintextOutput(e3_id, decrypted_output, proof)
                    .nonce(current_nonce);
                let receipt = builder.send().await?.get_receipt().await?;
                Ok(receipt)
            }
        },
    )
    .await
}

async fn process_e3_failure<P: Provider + WalletProvider + Clone>(
    provider: EthProvider<P>,
    contract_address: Address,
    e3_id: E3id,
) -> Result<TransactionReceipt> {
    let e3_id: U256 = e3_id.try_into()?;

    send_tx_with_retry("processE3Failure", &[], || {
        info!("processE3Failure() e3_id={:?}", e3_id);
        let provider = provider.clone();

        async move {
            let from_address = provider.provider().default_signer_address();
            let current_nonce = provider
                .provider()
                .get_transaction_count(from_address)
                .pending()
                .await?;
            let contract = IEnclave::new(contract_address, provider.provider());
            let builder = contract.processE3Failure(e3_id).nonce(current_nonce);
            let receipt = builder.send().await?.get_receipt().await?;
            Ok(receipt)
        }
    })
    .await
}
