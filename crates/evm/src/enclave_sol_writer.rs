// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::error_decoder::format_evm_error;
use crate::helpers::{encode_zk_proof, EthProvider};
use crate::send_tx_with_retry;
use actix::prelude::*;
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
use e3_events::BusHandle;
use e3_events::E3RequestComplete;
use e3_events::EnclaveEvent;
use e3_events::EnclaveEventData;
use e3_events::EventType;
use e3_events::Shutdown;
use e3_events::{prelude::*, AggregatorChanged, EffectsEnabled};
use e3_events::{E3Stage, E3StageChanged};
use e3_events::{E3id, EType, PlaintextAggregated, Proof};
use e3_utils::NotifySync;
use e3_utils::MAILBOX_LIMIT;
use std::collections::HashMap;
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
    bus: BusHandle,
    effects_enabled: bool,
    active_aggregators: HashMap<E3id, bool>,
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
            effects_enabled: false,
            active_aggregators: HashMap::new(),
        })
    }

    pub fn attach(bus: &BusHandle, provider: EthProvider<P>, contract_address: Address) {
        let addr = EnclaveSolWriter::new(bus, provider, contract_address)
            .expect("failed to create EnclaveSolWriter")
            .start();
        bus.subscribe_all(
            &[
                EventType::EffectsEnabled,
                EventType::AggregatorChanged,
                EventType::PlaintextAggregated,
                EventType::E3StageChanged,
                EventType::E3RequestComplete,
                EventType::Shutdown,
            ],
            addr.into(),
        );
    }

    fn is_active_aggregator_for(&self, e3_id: &E3id) -> bool {
        self.active_aggregators.get(e3_id).copied().unwrap_or(false)
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
            EnclaveEventData::EffectsEnabled(data) => self.notify_sync(ctx, data),
            EnclaveEventData::AggregatorChanged(data) => self.notify_sync(ctx, data),
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
            EnclaveEventData::E3RequestComplete(data) => self.notify_sync(ctx, data),
            EnclaveEventData::Shutdown(data) => self.notify_sync(ctx, data),
            _ => (),
        }
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<EffectsEnabled>
    for EnclaveSolWriter<P>
{
    type Result = ();

    fn handle(&mut self, _: EffectsEnabled, _: &mut Self::Context) -> Self::Result {
        self.effects_enabled = true;
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<AggregatorChanged>
    for EnclaveSolWriter<P>
{
    type Result = ();

    fn handle(&mut self, msg: AggregatorChanged, _: &mut Self::Context) -> Self::Result {
        self.active_aggregators.insert(msg.e3_id, msg.is_aggregator);
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<E3RequestComplete>
    for EnclaveSolWriter<P>
{
    type Result = ();

    fn handle(&mut self, msg: E3RequestComplete, _: &mut Self::Context) -> Self::Result {
        self.active_aggregators.remove(&msg.e3_id);
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<PlaintextAggregated>
    for EnclaveSolWriter<P>
{
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: PlaintextAggregated, _: &mut Self::Context) -> Self::Result {
        if !self.effects_enabled || !self.is_active_aggregator_for(&msg.e3_id) {
            return Box::pin(async {});
        }

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
                // `decryption_aggregator_proofs` is empty when proof aggregation is disabled.
                // When enabled, its length must match `decrypted_output`.
                if !msg.decryption_aggregator_proofs.is_empty()
                    && decrypted_output.len() != msg.decryption_aggregator_proofs.len()
                {
                    bus.err(
                        EType::Evm,
                        anyhow::anyhow!(
                            "E3 {} decrypted_output len ({}) != decryption_aggregator_proofs len ({})",
                            e3_id,
                            decrypted_output.len(),
                            msg.decryption_aggregator_proofs.len()
                        ),
                    );
                    return;
                }
                match should_publish_plaintext(provider.clone(), contract_address, e3_id.clone())
                    .await
                {
                    Ok(false) => {
                        info!(e3_id = %e3_id, "Skipping publishPlaintextOutput; plaintext already published");
                        return;
                    }
                    Err(err) => {
                        bus.err(
                            EType::Evm,
                            anyhow::anyhow!(
                                "Error preflighting plaintext publication: {}",
                                format_evm_error(&err)
                            ),
                        );
                        return;
                    }
                    Ok(true) => {}
                }

                let result = publish_plaintext_output(
                    provider,
                    contract_address,
                    e3_id,
                    decrypted.extract_bytes(),
                    msg.decryption_aggregator_proofs.first(),
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
        if !self.effects_enabled {
            return Box::pin(async {});
        }

        Box::pin({
            let e3_id = msg.e3_id.clone();
            let contract_address = self.contract_address;
            let provider = self.provider.clone();
            async move {
                let result = process_e3_failure(provider, contract_address, e3_id.clone()).await;
                match result {
                    Ok(receipt) => {
                        info!(
                            tx=%receipt.transaction_hash,
                            e3_id = %e3_id,
                            "Called processE3Failure"
                        );
                    }
                    Err(err) => {
                        info!(
                            e3_id = %e3_id,
                            "processE3Failure did not succeed (may already be processed): {}",
                            format_evm_error(&err)
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
    decryption_aggregator_proof: Option<&Proof>,
) -> Result<TransactionReceipt> {
    let e3_id: U256 = e3_id.try_into()?;

    let from_address = provider.provider().default_signer_address();
    let current_nonce = provider
        .provider()
        .get_transaction_count(from_address)
        .pending()
        .await?;

    // `None` => proof aggregation disabled; contract accepts empty bytes in that case.
    let proof: Bytes = match decryption_aggregator_proof {
        Some(p) => encode_zk_proof(p)?,
        None => Bytes::new(),
    };

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

async fn should_publish_plaintext<P: Provider + WalletProvider + Clone>(
    provider: EthProvider<P>,
    contract_address: Address,
    e3_id: E3id,
) -> Result<bool> {
    let e3_id: U256 = e3_id.try_into()?;
    let contract = IEnclave::new(contract_address, provider.provider());
    let e3 = contract.getE3(e3_id).call().await?;
    Ok(e3.plaintextOutput.is_empty())
}

async fn process_e3_failure<P: Provider + WalletProvider + Clone>(
    provider: EthProvider<P>,
    contract_address: Address,
    e3_id: E3id,
) -> Result<TransactionReceipt> {
    let e3_id: U256 = e3_id.try_into()?;

    info!("processE3Failure() e3_id={:?}", e3_id);

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
