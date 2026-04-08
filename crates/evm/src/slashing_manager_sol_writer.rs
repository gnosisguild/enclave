// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Subscribes to `AccusationQuorumReached` events and submits `proposeSlash`
//! transactions on the SlashingManager contract with committee attestation evidence.

use crate::error_decoder::format_evm_error;
use crate::helpers::EthProvider;
use crate::send_tx_with_retry;
use actix::prelude::*;
use actix::Addr;
use alloy::{
    primitives::{Address, Bytes, U256},
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
use std::time::Duration;
use tracing::{info, warn};

sol!(
    #[sol(rpc)]
    ISlashingManager,
    "../../packages/enclave-contracts/artifacts/contracts/interfaces/ISlashingManager.sol/ISlashingManager.json"
);

/// Maximum number of voters eligible to attempt on-chain submission.
/// Rank 0 submits immediately, rank 1 after one delay interval, etc.
const MAX_SLASH_SUBMITTERS: usize = 3;

/// Delay between fallback submission attempts (seconds).
/// Rank N waits N * SUBMITTER_DELAY_SECS before submitting.
const SUBMITTER_DELAY_SECS: u64 = 30;

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
                // 3. This node is among the top MAX_SLASH_SUBMITTERS voters
                //    (sorted ascending by address). The lowest-address voter
                //    submits immediately; higher-ranked fallback voters wait
                //    progressively longer (rank * SUBMITTER_DELAY_SECS) before
                //    attempting submission. On-chain DuplicateEvidence protection
                //    ensures at most one slash executes.
                let my_addr = self.provider.provider().default_signer_address();
                let mut sorted_voters: Vec<Address> =
                    data.votes_for.iter().map(|v| v.voter).collect();
                sorted_voters.sort();
                let my_rank = sorted_voters.iter().position(|&v| v == my_addr);

                if self.provider.chain_id() == data.e3_id.chain_id()
                    && matches!(
                        data.outcome,
                        AccusationOutcome::AccusedFaulted | AccusationOutcome::Equivocation
                    )
                    && my_rank.map_or(false, |r| r < MAX_SLASH_SUBMITTERS)
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
            let my_addr = self.provider.provider().default_signer_address();
            async move {
                // Compute this node's submission rank for staggered fallback
                let mut sorted_voters: Vec<Address> =
                    msg.votes_for.iter().map(|v| v.voter).collect();
                sorted_voters.sort();
                let rank = sorted_voters
                    .iter()
                    .position(|&v| v == my_addr)
                    .unwrap_or(0);

                // Fallback submitters wait before attempting, giving the primary
                // submitter time to land the transaction on-chain.
                if rank > 0 {
                    let delay = Duration::from_secs(rank as u64 * SUBMITTER_DELAY_SECS);
                    info!(
                        "Fallback submitter (rank {rank}): waiting {delay:?} before submission attempt"
                    );
                    tokio::time::sleep(delay).await;
                }

                let result = submit_slash_proposal(provider, contract_address, msg).await;
                match result {
                    Ok(receipt) => {
                        info!(tx=%receipt.transaction_hash, "Submitted attestation-based slash proposal on-chain");
                    }
                    Err(err) => {
                        let decoded = format_evm_error(&err);
                        if rank > 0 {
                            // Fallback submitters expect DuplicateEvidence reverts
                            // when the primary submitter has already landed the tx.
                            warn!(
                                "Fallback submitter (rank {rank}): slash submission failed \
                                 (likely already submitted by primary): {decoded}"
                            );
                        } else {
                            bus.err(
                                EType::Evm,
                                anyhow::anyhow!("Error submitting slash proposal: {decoded}"),
                            );
                        }
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
        .map(|v| Bytes::from(v.signature.extract_bytes()))
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

    let proof_data = encode_attestation_evidence(&data);

    send_tx_with_retry("proposeSlash", &[], || {
        info!("proposeSlash() e3_id={:?} operator={:?}", e3_id, operator);
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
                .proposeSlash(e3_id, operator, proof)
                .nonce(current_nonce);
            let receipt = builder.send().await?.get_receipt().await?;
            Ok(receipt)
        }
    })
    .await
}
