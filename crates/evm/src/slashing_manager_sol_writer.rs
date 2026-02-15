// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! FaultSubmitter actor — subscribes to `SignedProofFailed` events and submits
//! `proposeSlash` transactions on the SlashingManager contract.
//!
//! When a ciphernode broadcasts a ZK proof that fails local verification,
//! the `ProofVerificationActor` emits a `SignedProofFailed` event carrying the
//! self-authenticating evidence (the signed proof payload).  This actor consumes
//! that event, ABI-encodes the proof data, and calls `proposeSlash(e3Id, operator,
//! reason, proof)` on-chain.

use crate::helpers::EthProvider;
use crate::send_tx_with_retry;
use actix::prelude::*;
use actix::Addr;
use alloy::{
    primitives::{keccak256, Address, Bytes, U256},
    providers::{Provider, WalletProvider},
    rpc::types::TransactionReceipt,
    sol,
    sol_types::SolValue,
};
use anyhow::Result;
use e3_events::prelude::*;
use e3_events::BusHandle;
use e3_events::EnclaveEvent;
use e3_events::EnclaveEventData;
use e3_events::EventType;
use e3_events::Shutdown;
use e3_events::{EType, SignedProofFailed};
use e3_utils::NotifySync;
use tracing::info;

sol!(
    #[sol(rpc)]
    ISlashingManager,
    "../../packages/enclave-contracts/artifacts/contracts/interfaces/ISlashingManager.sol/ISlashingManager.json"
);

/// Consumes `SignedProofFailed` events and submits slash proposals on-chain.
pub struct SlashingManagerSolWriter<P> {
    provider: EthProvider<P>,
    contract_address: Address,
    bus: BusHandle,
}

impl<P: Provider + WalletProvider + Clone + 'static> SlashingManagerSolWriter<P> {
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

    pub async fn attach(
        bus: &BusHandle,
        provider: EthProvider<P>,
        contract_address: Address,
    ) -> Result<Addr<SlashingManagerSolWriter<P>>> {
        let addr = SlashingManagerSolWriter::new(bus, provider, contract_address)?.start();
        bus.subscribe_all(
            &[EventType::SignedProofFailed, EventType::Shutdown],
            addr.clone().into(),
        );
        Ok(addr)
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Actor for SlashingManagerSolWriter<P> {
    type Context = actix::Context<Self>;
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<EnclaveEvent>
    for SlashingManagerSolWriter<P>
{
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg.into_data() {
            EnclaveEventData::SignedProofFailed(data) => {
                // Only submit if the chain matches
                if self.provider.chain_id() == data.e3_id.chain_id() {
                    ctx.notify(data);
                }
            }
            EnclaveEventData::Shutdown(data) => self.notify_sync(ctx, data),
            _ => (),
        }
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<SignedProofFailed>
    for SlashingManagerSolWriter<P>
{
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: SignedProofFailed, _: &mut Self::Context) -> Self::Result {
        Box::pin({
            let contract_address = self.contract_address;
            let provider = self.provider.clone();
            let bus = self.bus.clone();
            async move {
                let result = submit_slash_proposal(provider, contract_address, msg).await;
                match result {
                    Ok(receipt) => {
                        info!(tx=%receipt.transaction_hash, "Submitted slash proposal on-chain");
                    }
                    Err(err) => {
                        bus.err(
                            EType::Evm,
                            anyhow::anyhow!("Error submitting slash proposal: {:?}", err),
                        );
                    }
                }
            }
        })
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<Shutdown>
    for SlashingManagerSolWriter<P>
{
    type Result = ();

    fn handle(&mut self, _: Shutdown, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

async fn submit_slash_proposal<P: Provider + WalletProvider + Clone>(
    provider: EthProvider<P>,
    contract_address: Address,
    data: SignedProofFailed,
) -> Result<TransactionReceipt> {
    let e3_id: U256 = data.e3_id.clone().try_into()?;
    let operator = data.faulting_node;
    let reason = keccak256(data.proof_type.slash_reason().as_bytes());

    // Encode the proof as (bytes zkProof, bytes32[] publicInputs) per SlashingManager.proposeSlash
    let zk_proof = Bytes::copy_from_slice(&data.signed_payload.payload.proof.data);
    let public_inputs_bytes = &data.signed_payload.payload.proof.public_signals;

    // Convert public signals to bytes32[] — each 32-byte chunk is one element
    let mut public_inputs: Vec<[u8; 32]> = Vec::new();
    for chunk in public_inputs_bytes.chunks(32) {
        let mut padded = [0u8; 32];
        let start = 32 - chunk.len();
        padded[start..].copy_from_slice(chunk);
        public_inputs.push(padded);
    }

    // abi.encode(bytes, bytes32[])
    let proof_data = (zk_proof, public_inputs).abi_encode();

    let from_address = provider.provider().default_signer_address();
    let current_nonce = provider
        .provider()
        .get_transaction_count(from_address)
        .pending()
        .await?;

    // DuplicateEvidence() = keccak256("DuplicateEvidence()")[:4] – retry if not yet on-chain
    send_tx_with_retry("proposeSlash", &[], || {
        info!(
            "proposeSlash() e3_id={:?} operator={:?} reason={:?}",
            e3_id, operator, reason
        );
        let proof = Bytes::from(proof_data.clone());
        let contract = ISlashingManager::new(contract_address, provider.provider());

        async move {
            let builder = contract
                .proposeSlash(e3_id, operator, reason.into(), proof)
                .nonce(current_nonce);
            let receipt = builder.send().await?.get_receipt().await?;
            Ok(receipt)
        }
    })
    .await
}
