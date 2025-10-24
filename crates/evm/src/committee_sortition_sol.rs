// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{helpers::EthProvider, EvmEventReader, EvmEventReaderState};
use actix::prelude::*;
use alloy::{
    primitives::{Address, LogData, B256, U256},
    providers::{Provider, WalletProvider},
    rpc::types::TransactionReceipt,
    sol,
    sol_types::SolEvent,
};
use anyhow::Result;
use e3_data::Repository;
use e3_events::{BusError, E3id, EnclaveErrorType, EnclaveEvent, EventBus, Shutdown, Subscribe};
use tracing::{error, info, trace};

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    CommitteeSortition,
    "../../packages/enclave-contracts/artifacts/contracts/sortition/CommitteeSortition.sol/CommitteeSortition.json"
);

struct TicketSubmittedWithChainId(pub CommitteeSortition::TicketSubmitted, pub u64);

impl From<TicketSubmittedWithChainId> for e3_events::TicketSubmitted {
    fn from(value: TicketSubmittedWithChainId) -> Self {
        e3_events::TicketSubmitted {
            e3_id: E3id::new(value.0.e3Id.to_string(), value.1),
            node: value.0.node.to_string(),
            ticket_number: value.0.ticketNumber.try_into().unwrap_or(0),
            score: value.0.score.to_string(),
            added_to_committee: value.0.addedToCommittee,
            chain_id: value.1,
        }
    }
}

impl From<TicketSubmittedWithChainId> for EnclaveEvent {
    fn from(value: TicketSubmittedWithChainId) -> Self {
        let payload: e3_events::TicketSubmitted = value.into();
        EnclaveEvent::from(payload)
    }
}

struct CommitteeFinalizedWithChainId(pub CommitteeSortition::CommitteeFinalized, pub u64);

impl From<CommitteeFinalizedWithChainId> for e3_events::CommitteeFinalized {
    fn from(value: CommitteeFinalizedWithChainId) -> Self {
        e3_events::CommitteeFinalized {
            e3_id: E3id::new(value.0.e3Id.to_string(), value.1),
            committee: value
                .0
                .committee
                .iter()
                .map(|addr| addr.to_string())
                .collect(),
            chain_id: value.1,
        }
    }
}

impl From<CommitteeFinalizedWithChainId> for EnclaveEvent {
    fn from(value: CommitteeFinalizedWithChainId) -> Self {
        let payload: e3_events::CommitteeFinalized = value.into();
        EnclaveEvent::from(payload)
    }
}

pub fn extractor(data: &LogData, topic: Option<&B256>, chain_id: u64) -> Option<EnclaveEvent> {
    match topic {
        Some(&CommitteeSortition::TicketSubmitted::SIGNATURE_HASH) => {
            let Ok(event) = CommitteeSortition::TicketSubmitted::decode_log_data(data) else {
                error!("Error parsing event TicketSubmitted after topic was matched!");
                return None;
            };
            Some(EnclaveEvent::from(TicketSubmittedWithChainId(
                event, chain_id,
            )))
        }
        Some(&CommitteeSortition::CommitteeFinalized::SIGNATURE_HASH) => {
            let Ok(event) = CommitteeSortition::CommitteeFinalized::decode_log_data(data) else {
                error!("Error parsing event CommitteeFinalized after topic was matched!");
                return None;
            };
            Some(EnclaveEvent::from(CommitteeFinalizedWithChainId(
                event, chain_id,
            )))
        }
        _topic => {
            trace!(
                topic=?_topic,
                "Unknown event was received by CommitteeSortition.sol parser but was ignored"
            );
            None
        }
    }
}

pub struct CommitteeSortitionSolReader;

impl CommitteeSortitionSolReader {
    pub async fn attach<P: Provider + Clone + 'static>(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: EthProvider<P>,
        contract_address: &str,
        repository: &Repository<EvmEventReaderState>,
        start_block: Option<u64>,
        rpc_url: String,
    ) -> Result<Addr<EvmEventReader<P>>> {
        let addr = EvmEventReader::attach(
            provider,
            extractor,
            contract_address,
            start_block,
            &bus.clone().into(),
            repository,
            rpc_url,
        )
        .await?;

        info!(address=%contract_address, "CommitteeSortitionSolReader is listening to address");

        Ok(addr)
    }
}

/// Writer for CommitteeSortition contract
pub struct CommitteeSortitionSolWriter<P> {
    provider: EthProvider<P>,
    contract_address: Address,
    bus: Addr<EventBus<EnclaveEvent>>,
}

impl<P: Provider + WalletProvider + Clone + 'static> CommitteeSortitionSolWriter<P> {
    pub fn new(
        bus: &Addr<EventBus<EnclaveEvent>>,
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
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: EthProvider<P>,
        contract_address: &str,
    ) -> Result<Addr<CommitteeSortitionSolWriter<P>>> {
        let addr =
            CommitteeSortitionSolWriter::new(bus, provider, contract_address.parse()?)?.start();

        bus.send(Subscribe::new("CiphernodeSelected", addr.clone().into()))
            .await?;

        bus.send(Subscribe::new("Shutdown", addr.clone().into()))
            .await?;

        Ok(addr)
    }

    async fn submit_ticket(&self, e3_id: E3id, ticket_number: u64) -> Result<TransactionReceipt> {
        let e3_id_u256: U256 = e3_id.clone().try_into()?;
        let ticket_number_u256 = U256::from(ticket_number);

        let from_address = self.provider.provider().default_signer_address();
        let current_nonce = self
            .provider
            .provider()
            .get_transaction_count(from_address)
            .pending()
            .await?;

        let contract = CommitteeSortition::new(self.contract_address, self.provider.provider());
        let builder = contract
            .submitTicket(e3_id_u256, ticket_number_u256)
            .nonce(current_nonce);

        let receipt = builder.send().await?.get_receipt().await?;
        Ok(receipt)
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Actor for CommitteeSortitionSolWriter<P> {
    type Context = actix::Context<Self>;
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<EnclaveEvent>
    for CommitteeSortitionSolWriter<P>
{
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::CiphernodeSelected { data, .. } => {
                ctx.notify(data);
            }
            EnclaveEvent::Shutdown { data, .. } => {
                ctx.notify(data);
            }
            _ => {}
        }
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<e3_events::CiphernodeSelected>
    for CommitteeSortitionSolWriter<P>
{
    type Result = ResponseFuture<()>;

    fn handle(
        &mut self,
        data: e3_events::CiphernodeSelected,
        _: &mut Self::Context,
    ) -> Self::Result {
        let e3_id = data.e3_id.clone();
        let provider = self.provider.clone();
        let contract_address = self.contract_address;
        let bus = self.bus.clone();

        let ticket_number = data.ticket_id;

        Box::pin(async move {
            let Some(ticket) = ticket_number else {
                info!(
                    "CiphernodeSelected: No ticket number (non-bonding backend), skipping ticket submission for E3 {:?}",
                    e3_id
                );
                return;
            };

            info!(
                "CiphernodeSelected: Submitting ticket {} for E3 {:?}",
                ticket, e3_id
            );

            // Get the node's wallet address
            let node_address = provider.provider().default_signer_address();

            info!(
                "Node {:?} submitting ticket {} for E3 {:?}",
                node_address, ticket, e3_id
            );

            let writer = CommitteeSortitionSolWriter::new(&bus, provider.clone(), contract_address)
                .expect("Failed to create writer");

            match writer.submit_ticket(e3_id.clone(), ticket).await {
                Ok(receipt) => {
                    info!(
                        "Successfully submitted ticket for E3 {:?}, tx: {:?}",
                        e3_id, receipt.transaction_hash
                    );
                }
                Err(e) => {
                    error!("Failed to submit ticket for E3 {:?}: {:?}", e3_id, e);
                    bus.err(EnclaveErrorType::Evm, e);
                }
            }
        })
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<Shutdown>
    for CommitteeSortitionSolWriter<P>
{
    type Result = ();

    fn handle(&mut self, _: Shutdown, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

/// Wrapper for reader and writer
pub struct CommitteeSortitionSol;

impl CommitteeSortitionSol {
    pub async fn attach<P>(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: EthProvider<P>,
        contract_address: &str,
        repository: &Repository<EvmEventReaderState>,
        start_block: Option<u64>,
        rpc_url: String,
    ) -> Result<Addr<CommitteeSortitionSolWriter<P>>>
    where
        P: Provider + WalletProvider + Clone + 'static,
    {
        CommitteeSortitionSolReader::attach(
            bus,
            provider.clone(),
            contract_address,
            repository,
            start_block,
            rpc_url,
        )
        .await?;

        let writer = CommitteeSortitionSolWriter::attach(bus, provider, contract_address).await?;

        Ok(writer)
    }
}
