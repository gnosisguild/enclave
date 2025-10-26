// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{event_reader::EvmEventReaderState, helpers::EthProvider, EvmEventReader};
use actix::prelude::*;
use alloy::{
    primitives::{Address, Bytes, LogData, B256, U256},
    providers::{Provider, WalletProvider},
    rpc::types::TransactionReceipt,
    sol,
    sol_types::SolEvent,
};
use anyhow::Result;
use e3_data::Repository;
use e3_events::{
    BusError, CiphernodeSelected, CommitteeFinalized, E3id, EnclaveErrorType, EnclaveEvent,
    EventBus, OrderedSet, PublicKeyAggregated, Seed, Shutdown, Subscribe,
};
use std::collections::HashMap;
use tracing::{error, info, trace};

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    ICiphernodeRegistry,
    "../../packages/enclave-contracts/artifacts/contracts/interfaces/ICiphernodeRegistry.sol/ICiphernodeRegistry.json"
);

struct CiphernodeAddedWithChainId(pub ICiphernodeRegistry::CiphernodeAdded, pub u64);

impl From<CiphernodeAddedWithChainId> for e3_events::CiphernodeAdded {
    fn from(value: CiphernodeAddedWithChainId) -> Self {
        e3_events::CiphernodeAdded {
            address: value.0.node.to_string(),
            // TODO: limit index and numNodes to uint32 at the solidity level
            index: value
                .0
                .index
                .try_into()
                .expect("Index exceeds usize capacity"),
            num_nodes: value
                .0
                .numNodes
                .try_into()
                .expect("NumNodes exceeds usize capacity"),
            chain_id: value.1,
        }
    }
}

impl From<CiphernodeAddedWithChainId> for EnclaveEvent {
    fn from(value: CiphernodeAddedWithChainId) -> Self {
        let payload: e3_events::CiphernodeAdded = value.into();
        EnclaveEvent::from(payload)
    }
}

struct CiphernodeRemovedWithChainId(pub ICiphernodeRegistry::CiphernodeRemoved, pub u64);

impl From<CiphernodeRemovedWithChainId> for e3_events::CiphernodeRemoved {
    fn from(value: CiphernodeRemovedWithChainId) -> Self {
        e3_events::CiphernodeRemoved {
            address: value.0.node.to_string(),
            index: value
                .0
                .index
                .try_into()
                .expect("Index exceeds usize capacity"),
            num_nodes: value
                .0
                .numNodes
                .try_into()
                .expect("NumNodes exceeds usize capacity"),
            chain_id: value.1,
        }
    }
}

impl From<CiphernodeRemovedWithChainId> for EnclaveEvent {
    fn from(value: CiphernodeRemovedWithChainId) -> Self {
        let payload: e3_events::CiphernodeRemoved = value.into();
        EnclaveEvent::from(payload)
    }
}

struct CommitteeRequestedWithChainId(pub ICiphernodeRegistry::CommitteeRequested, pub u64);

impl From<CommitteeRequestedWithChainId> for e3_events::CommitteeRequested {
    fn from(value: CommitteeRequestedWithChainId) -> Self {
        e3_events::CommitteeRequested {
            e3_id: E3id::new(value.0.e3Id.to_string(), value.1),
            seed: Seed(value.0.seed.to_be_bytes()),
            threshold: [value.0.threshold[0] as usize, value.0.threshold[1] as usize],
            request_block: value.0.requestBlock.to(),
            submission_deadline: value.0.submissionDeadline.to(),
            chain_id: value.1,
        }
    }
}

impl From<CommitteeRequestedWithChainId> for EnclaveEvent {
    fn from(value: CommitteeRequestedWithChainId) -> Self {
        let payload: e3_events::CommitteeRequested = value.into();
        EnclaveEvent::from(payload)
    }
}

struct CommitteeFinalizedWithChainId(pub ICiphernodeRegistry::CommitteeFinalized, pub u64);

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

struct TicketSubmittedWithChainId(pub ICiphernodeRegistry::TicketSubmitted, pub u64);

impl From<TicketSubmittedWithChainId> for e3_events::TicketSubmitted {
    fn from(value: TicketSubmittedWithChainId) -> Self {
        e3_events::TicketSubmitted {
            e3_id: E3id::new(value.0.e3Id.to_string(), value.1),
            node: value.0.node.to_string(),
            ticket_id: value.0.ticketId.to(),
            score: value.0.score.to_string(),
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

pub fn extractor(data: &LogData, topic: Option<&B256>, chain_id: u64) -> Option<EnclaveEvent> {
    match topic {
        Some(&ICiphernodeRegistry::CiphernodeAdded::SIGNATURE_HASH) => {
            let Ok(event) = ICiphernodeRegistry::CiphernodeAdded::decode_log_data(data) else {
                error!("Error parsing event CiphernodeAdded after topic was matched!");
                return None;
            };
            Some(EnclaveEvent::from(CiphernodeAddedWithChainId(
                event, chain_id,
            )))
        }
        Some(&ICiphernodeRegistry::CiphernodeRemoved::SIGNATURE_HASH) => {
            let Ok(event) = ICiphernodeRegistry::CiphernodeRemoved::decode_log_data(data) else {
                error!("Error parsing event CiphernodeRemoved after topic was matched!");
                return None;
            };
            Some(EnclaveEvent::from(CiphernodeRemovedWithChainId(
                event, chain_id,
            )))
        }
        Some(&ICiphernodeRegistry::CommitteeRequested::SIGNATURE_HASH) => {
            let Ok(event) = ICiphernodeRegistry::CommitteeRequested::decode_log_data(data) else {
                error!("Error parsing event CommitteeRequested after topic was matched!");
                return None;
            };
            Some(EnclaveEvent::from(CommitteeRequestedWithChainId(
                event, chain_id,
            )))
        }
        Some(&ICiphernodeRegistry::CommitteeFinalized::SIGNATURE_HASH) => {
            let Ok(event) = ICiphernodeRegistry::CommitteeFinalized::decode_log_data(data) else {
                error!("Error parsing event CommitteeFinalized after topic was matched!");
                return None;
            };
            Some(EnclaveEvent::from(CommitteeFinalizedWithChainId(
                event, chain_id,
            )))
        }
        Some(&ICiphernodeRegistry::TicketSubmitted::SIGNATURE_HASH) => {
            let Ok(event) = ICiphernodeRegistry::TicketSubmitted::decode_log_data(data) else {
                error!("Error parsing event TicketSubmitted after topic was matched!");
                return None;
            };
            Some(EnclaveEvent::from(TicketSubmittedWithChainId(
                event, chain_id,
            )))
        }
        _topic => {
            trace!(
                topic=?_topic,
                "Unknown event was received by CiphernodeRegistry.sol parser but was ignored"
            );
            None
        }
    }
}

/// Connects to CiphernodeRegistry.sol converting EVM events to EnclaveEvents
pub struct CiphernodeRegistrySolReader;

impl CiphernodeRegistrySolReader {
    pub async fn attach<P>(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: EthProvider<P>,
        contract_address: &str,
        repository: &Repository<EvmEventReaderState>,
        start_block: Option<u64>,
        rpc_url: String,
    ) -> Result<Addr<EvmEventReader<P>>>
    where
        P: Provider + Clone + 'static,
    {
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

        info!(address=%contract_address, "CiphernodeRegistrySolReader is listening to address");

        Ok(addr)
    }
}

/// Writer for publishing committees to CiphernodeRegistry
pub struct CiphernodeRegistrySolWriter<P> {
    provider: EthProvider<P>,
    contract_address: Address,
    bus: Addr<EventBus<EnclaveEvent>>,
    /// Store finalized committees for score sortition
    /// Maps E3id to the finalized committee nodes
    finalized_committees: HashMap<E3id, Vec<String>>,
}

impl<P: Provider + WalletProvider + Clone + 'static> CiphernodeRegistrySolWriter<P> {
    pub async fn new(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: EthProvider<P>,
        contract_address: Address,
    ) -> Result<Self> {
        Ok(Self {
            provider,
            contract_address,
            bus: bus.clone(),
            finalized_committees: HashMap::new(),
        })
    }

    pub async fn attach(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: EthProvider<P>,
        contract_address: &str,
    ) -> Result<Addr<CiphernodeRegistrySolWriter<P>>> {
        let addr = CiphernodeRegistrySolWriter::new(bus, provider, contract_address.parse()?)
            .await?
            .start();

        let _ = bus
            .send(Subscribe::new("PublicKeyAggregated", addr.clone().into()))
            .await;

        // Subscribe to CommitteeFinalized for score sortition
        let _ = bus
            .send(Subscribe::new("CommitteeFinalized", addr.clone().into()))
            .await;

        // Subscribe to CiphernodeSelected for ticket submission
        let _ = bus
            .send(Subscribe::new("CiphernodeSelected", addr.clone().into()))
            .await;

        Ok(addr)
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Actor for CiphernodeRegistrySolWriter<P> {
    type Context = actix::Context<Self>;
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<EnclaveEvent>
    for CiphernodeRegistrySolWriter<P>
{
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::PublicKeyAggregated { data, .. } => {
                // Only publish if the src and destination chains match
                if self.provider.chain_id() == data.e3_id.chain_id() {
                    ctx.notify(data);
                }
            }
            EnclaveEvent::CommitteeFinalized { data, .. } => {
                // Only store if chain matches
                if self.provider.chain_id() == data.e3_id.chain_id() {
                    ctx.notify(data);
                }
            }
            EnclaveEvent::CiphernodeSelected { data, .. } => {
                // Submit ticket if chain matches and ticket_id is present
                if self.provider.chain_id() == data.e3_id.chain_id() {
                    ctx.notify(data);
                }
            }
            EnclaveEvent::Shutdown { data, .. } => ctx.notify(data),
            _ => (),
        }
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<CommitteeFinalized>
    for CiphernodeRegistrySolWriter<P>
{
    type Result = ();

    fn handle(&mut self, msg: CommitteeFinalized, _: &mut Self::Context) -> Self::Result {
        info!(
            "Storing finalized committee for E3 {:?} (score sortition)",
            msg.e3_id
        );
        self.finalized_committees
            .insert(msg.e3_id.clone(), msg.committee);
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<CiphernodeSelected>
    for CiphernodeRegistrySolWriter<P>
{
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: CiphernodeSelected, _: &mut Self::Context) -> Self::Result {
        let e3_id = msg.e3_id.clone();
        let ticket_id = msg.ticket_id;
        let contract_address = self.contract_address;
        let provider = self.provider.clone();
        let bus = self.bus.clone();

        Box::pin(async move {
            // Only submit if we have a ticket_id (score sortition)
            let Some(ticket_id) = ticket_id else {
                info!(
                    "No ticket_id for E3 {:?}, skipping ticket submission (distance sortition)",
                    e3_id
                );
                return;
            };

            info!("Submitting ticket {} for E3 {:?}", ticket_id, e3_id);

            let result =
                submit_ticket_to_registry(provider, contract_address, e3_id, ticket_id).await;
            match result {
                Ok(receipt) => {
                    info!(tx=%receipt.transaction_hash, "Ticket submitted to registry");
                }
                Err(err) => {
                    error!("Failed to submit ticket: {:?}", err);
                    bus.err(EnclaveErrorType::Evm, err);
                }
            }
        })
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<PublicKeyAggregated>
    for CiphernodeRegistrySolWriter<P>
{
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: PublicKeyAggregated, _: &mut Self::Context) -> Self::Result {
        let e3_id = msg.e3_id.clone();
        let pubkey = msg.pubkey.clone();
        let contract_address = self.contract_address;
        let provider = self.provider.clone();
        let bus = self.bus.clone();

        // Check if we have a finalized committee for this E3 (score sortition)
        // Otherwise use the nodes from PublicKeyAggregated (distance sortition)
        let nodes = if let Some(finalized_nodes) = self.finalized_committees.remove(&e3_id) {
            info!(
                "Using finalized committee nodes for E3 {:?} (score sortition)",
                e3_id
            );
            // Convert Vec<String> to OrderedSet<String>
            OrderedSet::from_iter(finalized_nodes)
        } else {
            info!(
                "Using aggregated nodes for E3 {:?} (distance sortition)",
                e3_id
            );
            msg.nodes.clone()
        };

        Box::pin(async move {
            let result =
                publish_committee_to_registry(provider, contract_address, e3_id, nodes, pubkey)
                    .await;
            match result {
                Ok(receipt) => {
                    info!(tx=%receipt.transaction_hash, "Committee published to registry");
                }
                Err(err) => bus.err(EnclaveErrorType::Evm, err),
            }
        })
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Handler<Shutdown>
    for CiphernodeRegistrySolWriter<P>
{
    type Result = ();

    fn handle(&mut self, _: Shutdown, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

pub async fn submit_ticket_to_registry<P: Provider + WalletProvider + Clone>(
    provider: EthProvider<P>,
    contract_address: Address,
    e3_id: E3id,
    ticket_number: u64,
) -> Result<TransactionReceipt> {
    let e3_id: U256 = e3_id.try_into()?;
    let ticket_number = U256::from(ticket_number);
    let from_address = provider.provider().default_signer_address();
    let current_nonce = provider
        .provider()
        .get_transaction_count(from_address)
        .pending()
        .await?;
    let contract = ICiphernodeRegistry::new(contract_address, provider.provider());
    let builder = contract
        .submitTicket(e3_id, ticket_number)
        .nonce(current_nonce);
    let receipt = builder.send().await?.get_receipt().await?;
    Ok(receipt)
}

pub async fn publish_committee_to_registry<P: Provider + WalletProvider + Clone>(
    provider: EthProvider<P>,
    contract_address: Address,
    e3_id: E3id,
    nodes: OrderedSet<String>,
    public_key: Vec<u8>,
) -> Result<TransactionReceipt> {
    let e3_id: U256 = e3_id.try_into()?;
    let public_key = Bytes::from(public_key);
    let nodes_vec: Vec<Address> = nodes
        .into_iter()
        .filter_map(|node| node.parse().ok())
        .collect();
    let from_address = provider.provider().default_signer_address();
    let current_nonce = provider
        .provider()
        .get_transaction_count(from_address)
        .pending()
        .await?;
    let contract = ICiphernodeRegistry::new(contract_address, provider.provider());
    let builder = contract
        .publishCommittee(e3_id, nodes_vec, public_key)
        .nonce(current_nonce);
    let receipt = builder.send().await?.get_receipt().await?;
    Ok(receipt)
}

/// Wrapper for a reader and writer
pub struct CiphernodeRegistrySol;

impl CiphernodeRegistrySol {
    pub async fn attach<P>(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: EthProvider<P>,
        contract_address: &str,
        repository: &Repository<EvmEventReaderState>,
        start_block: Option<u64>,
        rpc_url: String,
    ) -> Result<()>
    where
        P: Provider + Clone + 'static,
    {
        CiphernodeRegistrySolReader::attach(
            bus,
            provider,
            contract_address,
            repository,
            start_block,
            rpc_url,
        )
        .await?;
        Ok(())
    }

    pub async fn attach_writer<P>(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: EthProvider<P>,
        contract_address: &str,
    ) -> Result<()>
    where
        P: Provider + WalletProvider + Clone + 'static,
    {
        CiphernodeRegistrySolWriter::attach(bus, provider, contract_address).await?;
        Ok(())
    }
}
