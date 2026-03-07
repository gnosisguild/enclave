// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Subscribes to `AccusationQuorumReached` events and submits `proposeSlash`
//! transactions on the SlashingManager contract with committee attestation evidence.

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
use e3_events::{AccusationOutcome, AccusationQuorumReached, EType};
use e3_utils::NotifySync;
use tracing::info;

sol!(
    #[sol(rpc)]
    ISlashingManager,
    "../../packages/enclave-contracts/artifacts/contracts/interfaces/ISlashingManager.sol/ISlashingManager.json"
);

/// Submits `AccusationQuorumReached` events as slash proposals on-chain.
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
            &[EventType::AccusationQuorumReached, EventType::Shutdown],
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
            EnclaveEventData::AccusationQuorumReached(data) => {
                // Only submit if:
                // 1. This is the right chain
                // 2. The quorum decided the accused is at fault OR equivocated
                // 3. This node is the designated submitter (lowest-address agreeing
                //    voter). This is deterministic so exactly one node submits,
                //    and decouples submission from the accuser to avoid single-point
                //    failures if the accuser's node goes down after quorum.
                let my_addr = self.provider.provider().default_signer_address();
                let is_designated_submitter = data
                    .votes_for
                    .iter()
                    .map(|v| v.voter)
                    .min()
                    .map_or(false, |min_voter| min_voter == my_addr);

                if self.provider.chain_id() == data.e3_id.chain_id()
                    && matches!(
                        data.outcome,
                        AccusationOutcome::AccusedFaulted | AccusationOutcome::Equivocation
                    )
                    && is_designated_submitter
                {
                    ctx.notify(data);
                }
            }
            EnclaveEventData::Shutdown(data) => self.notify_sync(ctx, data),
            _ => (),
        }
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<AccusationQuorumReached>
    for SlashingManagerSolWriter<P>
{
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: AccusationQuorumReached, _: &mut Self::Context) -> Self::Result {
        Box::pin({
            let contract_address = self.contract_address;
            let provider = self.provider.clone();
            let bus = self.bus.clone();
            async move {
                let result = submit_slash_proposal(provider, contract_address, msg).await;
                match result {
                    Ok(receipt) => {
                        info!(tx=%receipt.transaction_hash, "Submitted attestation-based slash proposal on-chain");
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

/// Encode `AccusationQuorumReached` into the attestation evidence format expected
/// by `SlashingManager.proposeSlash()`:
/// `abi.encode(uint256 proofType, address[] voters, bool[] agrees, bytes32[] dataHashes, bytes[] signatures)`
///
/// Voters are sorted ascending by address to satisfy the contract's duplicate-prevention check.
fn encode_attestation_evidence(data: &AccusationQuorumReached) -> Vec<u8> {
    // Collect and sort votes by voter address (ascending)
    let mut votes = data.votes_for.clone();
    votes.sort_by_key(|v| v.voter);

    let proof_type = U256::from(data.proof_type as u8);
    let voters: Vec<Address> = votes.iter().map(|v| v.voter).collect();
    let agrees: Vec<bool> = votes.iter().map(|v| v.agrees).collect();
    let data_hashes: Vec<[u8; 32]> = votes.iter().map(|v| v.data_hash).collect();
    let signatures: Vec<Bytes> = votes
        .iter()
        .map(|v| Bytes::from(v.signature.clone()))
        .collect();

    (proof_type, voters, agrees, data_hashes, signatures).abi_encode()
}

async fn submit_slash_proposal<P: Provider + WalletProvider + Clone>(
    provider: EthProvider<P>,
    contract_address: Address,
    data: AccusationQuorumReached,
) -> Result<TransactionReceipt> {
    let e3_id: U256 = data.e3_id.clone().try_into()?;
    let operator = data.accused;
    let reason = keccak256(data.proof_type.slash_reason().as_bytes());

    let proof_data = encode_attestation_evidence(&data);

    send_tx_with_retry("proposeSlash", &[], || {
        info!(
            "proposeSlash() e3_id={:?} operator={:?} reason={:?}",
            e3_id, operator, reason
        );
        let proof = Bytes::from(proof_data.clone());
        let provider = provider.clone();

        async move {
            let from_address = provider.provider().default_signer_address();
            let current_nonce = provider
                .provider()
                .get_transaction_count(from_address)
                .pending()
                .await?;
            let contract = ISlashingManager::new(contract_address, provider.provider());
            let builder = contract
                .proposeSlash(e3_id, operator, reason.into(), proof)
                .nonce(current_nonce);
            let receipt = builder.send().await?.get_receipt().await?;
            Ok(receipt)
        }
    })
    .await
}
