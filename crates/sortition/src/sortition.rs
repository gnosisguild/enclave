// SPDX-License-Identifier: LGPL-4.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::backends::{SortitionBackend, SortitionList};
use crate::ticket_sortition;
use crate::CiphernodeSelector;
use actix::prelude::*;
use alloy::primitives::U256;
use anyhow::{anyhow, Result};
use e3_data::{AutoPersist, Persistable, Repository};
use e3_events::{
    prelude::*, trap, CiphernodeAdded, CiphernodeRemoved, Committee, CommitteeFinalized,
    CommitteeMemberExpelled, CommitteePublished, ConfigurationUpdated, E3Failed, E3Requested,
    E3Stage, E3StageChanged, EType, EnclaveEvent, EventContext, EventType,
    OperatorActivationChanged, PlaintextOutputPublished, Seed, Sequenced, TicketBalanceUpdated,
    TypedEvent,
};
use e3_events::{BusHandle, E3id, EnclaveEventData};
use e3_utils::{NotifySync, MAILBOX_LIMIT};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;
use tracing::{info, instrument, warn};

/// State for a single ciphernode
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeState {
    /// Current ticket balance for this node
    pub ticket_balance: U256,
    /// Number of active E3 jobs this node is currently participating in
    pub active_jobs: u64,
    /// Whether this node is active (has met minimum requirements)
    pub active: bool,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            ticket_balance: U256::ZERO,
            active_jobs: 0,
            active: false,
        }
    }
}

/// Unified state for all nodes across all chains
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NodeStateStore {
    /// Map of node_address to node state
    pub nodes: HashMap<String, NodeState>,
    /// Current ticket price
    pub ticket_price: U256,
    /// Map of E3 ID to the committee nodes for that E3
    /// This is used to track which nodes are in which E3 jobs
    pub e3_committees: HashMap<String, Vec<String>>,
}

impl NodeStateStore {
    /// Get available tickets for a node, accounting for active jobs
    /// The Process for calculating available tickets is:
    /// 1. Get the node state for the node
    /// 2. Check if the node is active
    /// 3. Check if the node has a ticket price
    /// 4. Check if the node has a ticket balance
    /// 5. Calculate the available tickets
    /// 6. Subtract the active jobs from the available tickets
    /// 7. Return the available tickets
    pub fn available_tickets(&self, address: &str) -> u64 {
        if self.ticket_price.is_zero() {
            warn!("Ticket price is zero, returning 0 tickets, Please make sure this is the correct behavior");
            return 0;
        }

        let node = self.nodes.get(address);

        if let Some(node) = node {
            let total_tickets = (node.ticket_balance / self.ticket_price)
                .try_into()
                .unwrap_or(0u64);
            total_tickets.saturating_sub(node.active_jobs)
        } else {
            0
        }
    }

    /// Get all nodes with their available tickets
    /// Only includes active nodes
    pub fn get_nodes_with_tickets(&self) -> Vec<(String, u64)> {
        self.nodes
            .iter()
            .filter(|(_, node_state)| node_state.active)
            .map(|(addr, _)| (addr.clone(), self.available_tickets(addr)))
            .filter(|(_, tickets)| *tickets > 0)
            .collect()
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct WithSortitionTicket<T> {
    inner: T,
    party_ticket_id: Option<(u64, Option<u64>)>,
    address: String,
}

impl<T> WithSortitionTicket<T> {
    pub fn new(inner: T, party_ticket_id: Option<(u64, Option<u64>)>, address: &str) -> Self {
        Self {
            inner,
            party_ticket_id,
            address: address.to_owned(),
        }
    }

    pub fn is_selected(&self) -> bool {
        self.party_ticket_id.is_some()
    }

    pub fn address(&self) -> &str {
        self.address.as_ref()
    }

    pub fn ticket_id(&self) -> Option<u64> {
        self.party_ticket_id.and_then(|(_, ticket_id)| ticket_id)
    }

    pub fn party_id(&self) -> Option<u64> {
        self.party_ticket_id.map(|(party_id, _)| party_id)
    }
}

impl<T> Deref for WithSortitionTicket<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct E3CommitteeContainsRequest<T: Send + Sync>
where
    T: Send + Sync,
{
    inner: T,
    e3_id: E3id,
    node: String,
    sender: Recipient<E3CommitteeContainsResponse<T>>,
}

impl<T> E3CommitteeContainsRequest<T>
where
    T: Send + Sync,
{
    pub fn new(
        e3_id: E3id,
        node: String,
        inner: T,
        sender: impl Into<Recipient<E3CommitteeContainsResponse<T>>>,
    ) -> Self {
        Self {
            inner,
            e3_id,
            node,
            sender: sender.into(),
        }
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct E3CommitteeContainsResponse<T: Send + Sync> {
    inner: T,
    is_found_in_committee: bool,
}

impl<T> E3CommitteeContainsResponse<T>
where
    T: Send + Sync,
{
    pub fn new(inner: T, is_found_in_committee: bool) -> Self {
        Self {
            inner,
            is_found_in_committee,
        }
    }

    pub fn is_found_in_committee(&self) -> bool {
        self.is_found_in_committee
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Send + Sync> Deref for E3CommitteeContainsResponse<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Sortition actor that manages the sortition algorithm and the node state.
pub struct Sortition {
    /// Persistent map of `chain_id -> SortitionBackend`.
    backends: Persistable<HashMap<u64, SortitionBackend>>,
    /// Persistent map of `chain_id -> NodeStateStore`.
    node_state: Persistable<HashMap<u64, NodeStateStore>>,
    /// Event bus for error reporting and enclave event subscription.
    bus: BusHandle,
    /// Persistent map of finalized committees per E3
    finalized_committees: Persistable<HashMap<e3_events::E3id, Committee>>,
    /// Address for the CiphernodeSelector
    ciphernode_selector: Addr<CiphernodeSelector>,
    /// Address for the current node
    address: String,
}

/// Parameters for constructing a `Sortition` actor.
#[derive(Debug)]
pub struct SortitionParams {
    /// Event bus address.
    pub bus: BusHandle,
    /// Persisted per-chain backend map.
    pub backends: Persistable<HashMap<u64, SortitionBackend>>,
    /// Node state store per chain
    pub node_state: Persistable<HashMap<u64, NodeStateStore>>,
    /// Persistent map of finalized committees per E3
    pub finalized_committees: Persistable<HashMap<e3_events::E3id, Committee>>,
    /// Address for the CiphernodeSelector
    pub ciphernode_selector: Addr<CiphernodeSelector>,
    /// Address for the current node
    pub address: String,
}

impl Sortition {
    pub fn new(params: SortitionParams) -> Self {
        Self {
            backends: params.backends,
            node_state: params.node_state,
            bus: params.bus,
            finalized_committees: params.finalized_committees,
            ciphernode_selector: params.ciphernode_selector,
            address: params.address,
        }
    }

    #[instrument(name = "sortition_attach", skip_all)]
    pub async fn attach(
        bus: &BusHandle,
        backends_store: Repository<HashMap<u64, SortitionBackend>>,
        node_state_store: Repository<HashMap<u64, NodeStateStore>>,
        committees_store: Repository<HashMap<e3_events::E3id, Committee>>,
        default_backend: SortitionBackend,
        ciphernode_selector: Addr<CiphernodeSelector>,
        address: &str,
    ) -> Result<Addr<Self>> {
        let mut backends = backends_store.load_or_default(HashMap::new()).await?;
        let node_state = node_state_store.load_or_default(HashMap::new()).await?;
        let finalized_committees = committees_store.load_or_default(HashMap::new()).await?;

        backends.try_mutate_without_context(|mut list| {
            list.insert(u64::MAX, default_backend);
            Ok(list)
        })?;

        let addr = Sortition::new(SortitionParams {
            bus: bus.clone(),
            backends,
            node_state,
            finalized_committees,
            ciphernode_selector,
            address: address.to_owned(),
        })
        .start();

        // Subscribe to all relevant events
        bus.subscribe_all(
            &[
                EventType::E3Requested,
                EventType::CiphernodeAdded,
                EventType::CiphernodeRemoved,
                EventType::TicketBalanceUpdated,
                EventType::OperatorActivationChanged,
                EventType::ConfigurationUpdated,
                EventType::CommitteePublished,
                EventType::PlaintextOutputPublished,
                EventType::CommitteeFinalized,
                EventType::CommitteeMemberExpelled,
                EventType::E3Failed,
                EventType::E3StageChanged,
            ],
            addr.clone().into(),
        );

        info!("Sortition actor started");
        Ok(addr)
    }

    pub fn get_nodes(&self, chain_id: u64) -> Result<Vec<String>> {
        let map = self
            .backends
            .get()
            .ok_or_else(|| anyhow!("Could not get backends cache"))?;
        let backend = map
            .get(&chain_id)
            .ok_or_else(|| anyhow!("No backend for chain_id {}", chain_id))?;
        Ok(backend.nodes())
    }

    pub fn get_node_index(
        &self,
        e3_id: E3id,
        seed: Seed,
        size: usize,
        chain_id: u64,
    ) -> Option<(u64, Option<u64>)> {
        let bus = self.bus.clone();
        let map = self.backends.get()?;
        let state_map = self.node_state.get()?;
        let backend = map.get(&chain_id)?;
        let state = state_map.get(&chain_id)?;

        backend
            .get_index(e3_id, seed, size, self.address.clone(), chain_id, state)
            .unwrap_or_else(|err| {
                bus.err(EType::Sortition, err);
                None
            })
    }

    fn get_committee(&self, e3_id: &E3id) -> Option<Committee> {
        self.finalized_committees
            .get()
            .and_then(|committees| committees.get(e3_id).cloned())
    }

    fn committee_contains(&mut self, e3_id: E3id, node: String) -> bool {
        let Some(committee) = self.get_committee(&e3_id) else {
            // Non blocking error
            self.bus.err(
                EType::Sortition,
                anyhow!("No finalized committee found for E3 {}", e3_id),
            );
            return false;
        };

        committee.contains(&node)
    }
    /// Helper method to decrement active jobs for an E3's committee
    fn decrement_jobs_for_e3(
        &mut self,
        e3_id: &E3id,
        reason: &str,
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        self.node_state.try_mutate(&ec, |mut state_map| {
            let chain_id = e3_id.chain_id();
            let e3_id_str = format!("{}:{}", chain_id, e3_id.e3_id());

            if let Some(chain_state) = state_map.get_mut(&chain_id) {
                if let Some(committee_nodes) = chain_state.e3_committees.remove(&e3_id_str) {
                    // Decrement active jobs for each node in the committee
                    for node_addr in &committee_nodes {
                        if let Some(node) = chain_state.nodes.get_mut(node_addr) {
                            node.active_jobs = node.active_jobs.saturating_sub(1);

                            info!(
                                node = %node_addr,
                                chain_id = chain_id,
                                e3_id = ?e3_id,
                                active_jobs = node.active_jobs,
                                reason = reason,
                                "Decremented active jobs for node"
                            );
                        }
                    }

                    info!(
                        e3_id = ?e3_id,
                        committee_size = committee_nodes.len(),
                        reason = reason,
                        "E3 completed/failed - decremented active jobs for committee"
                    );
                } else {
                    info!(
                        e3_id = ?e3_id,
                        reason = reason,
                        "No committee found (might have been completed already)"
                    );
                }
            }

            Ok(state_map)
        })
    }
}

impl Actor for Sortition {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
    }
}

impl Handler<EnclaveEvent> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        match msg {
            EnclaveEventData::E3Requested(data) => self.notify_sync(ctx, TypedEvent::new(data, ec)),
            EnclaveEventData::CiphernodeAdded(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::CiphernodeRemoved(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::TicketBalanceUpdated(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::OperatorActivationChanged(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ConfigurationUpdated(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::CommitteePublished(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::PlaintextOutputPublished(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::CommitteeFinalized(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::CommitteeMemberExpelled(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            _ => (),
        }
    }
}

impl Handler<TypedEvent<E3Requested>> for Sortition {
    type Result = ();
    fn handle(&mut self, msg: TypedEvent<E3Requested>, _: &mut Self::Context) -> Self::Result {
        let e3_id = msg.e3_id.clone();
        let chain_id = msg.e3_id.chain_id();
        let seed = msg.seed;
        let threshold_m = msg.threshold_m;
        let threshold_n = msg.threshold_n;
        let buffer = ticket_sortition::calculate_buffer_size(threshold_m, threshold_n);
        let total_selection_size = threshold_n + buffer;

        info!(
            e3_id = %e3_id,
            threshold_m = threshold_m,
            threshold_n = threshold_n,
            buffer = buffer,
            total_selection_size = total_selection_size,
            "Performing Sortition with buffer"
        );

        let node_index = self.get_node_index(e3_id.clone(), seed, total_selection_size, chain_id);

        match &node_index {
            Some((index, ticket_id)) => {
                info!(
                    e3_id = %e3_id,
                    node = %self.address,
                    index = index,
                    ticket_id = ?ticket_id,
                    "This node was SELECTED for sortition"
                );
            }
            None => {
                info!(
                    e3_id = %e3_id,
                    node = %self.address,
                    "This node was NOT selected for sortition"
                );
            }
        }

        self.ciphernode_selector
            .do_send(WithSortitionTicket::new(msg, node_index, &self.address))
    }
}

impl Handler<TypedEvent<CiphernodeAdded>> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: TypedEvent<CiphernodeAdded>, _: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        trap(EType::Sortition, &self.bus.with_ec(&ec), || {
            let chain_id = msg.chain_id;
            let addr = msg.address.clone();

            self.node_state.try_mutate(&ec, |mut state_map| {
                let chain_state = state_map
                    .entry(chain_id)
                    .or_insert_with(NodeStateStore::default);
                chain_state
                    .nodes
                    .entry(addr.clone())
                    .or_insert_with(NodeState::default);
                Ok(state_map)
            })?;
            self.backends.try_mutate(&ec, move |mut list_map| {
                let default_backend = list_map
                    .get(&u64::MAX)
                    .cloned()
                    .unwrap_or_else(|| SortitionBackend::score());

                list_map
                    .entry(chain_id)
                    .or_insert_with(|| default_backend)
                    .add(addr);
                Ok(list_map)
            })?;
            info!(address = %msg.address, chain_id = chain_id, "Node added to sortition state");
            Ok(())
        })
    }
}

impl Handler<TypedEvent<CiphernodeRemoved>> for Sortition {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<CiphernodeRemoved>,
        _: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        trap(EType::Sortition, &self.bus.with_ec(&ec), || {
            let chain_id = msg.chain_id;
            let addr = msg.address.clone();

            self.node_state.try_mutate(&ec, |mut state_map| {
                if let Some(chain_state) = state_map.get_mut(&chain_id) {
                    chain_state.nodes.remove(&addr);
                }
                Ok(state_map)
            })?;
            self.backends.try_mutate(&ec, move |mut list_map| {
                if let Some(backend) = list_map.get_mut(&chain_id) {
                    backend.remove(addr);
                }
                Ok(list_map)
            })?;
            info!(address = %msg.address, chain_id = chain_id, "Node removed from sortition state");
            Ok(())
        })
    }
}

impl Handler<TypedEvent<TicketBalanceUpdated>> for Sortition {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<TicketBalanceUpdated>,
        _: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        trap(EType::Sortition, &self.bus.with_ec(&ec), || {
            self.node_state.try_mutate(&ec, |mut state_map| {
                let chain_state = state_map
                    .entry(msg.chain_id)
                    .or_insert_with(NodeStateStore::default);
                let node = chain_state
                    .nodes
                    .entry(msg.operator.clone())
                    .or_insert_with(NodeState::default);
                node.ticket_balance = msg.new_balance;

                info!(
                    operator = %msg.operator,
                    chain_id = msg.chain_id,
                    new_balance = ?msg.new_balance,
                    "Updated ticket balance"
                );

                Ok(state_map)
            })
        })
    }
}

impl Handler<TypedEvent<OperatorActivationChanged>> for Sortition {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<OperatorActivationChanged>,
        _: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        trap(EType::Sortition, &self.bus.with_ec(&ec), || {
            self.node_state.try_mutate(&ec, |mut state_map| {
                // Update all entries for this operator across all chains
                for (_, chain_state) in state_map.iter_mut() {
                    let node = chain_state
                        .nodes
                        .entry(msg.operator.clone())
                        .or_insert_with(NodeState::default);

                    node.active = msg.active;

                    info!(
                        operator = %msg.operator,
                        active = msg.active,
                        "Updated operator active status"
                    );
                }
                Ok(state_map)
            })
        })
    }
}

impl Handler<TypedEvent<ConfigurationUpdated>> for Sortition {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ConfigurationUpdated>,
        _: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        trap(EType::Sortition, &self.bus.with_ec(&ec), || {
            if msg.parameter == "ticketPrice" {
                self.node_state.try_mutate(&ec, |mut state_map| {
                    let chain_state = state_map
                        .entry(msg.chain_id)
                        .or_insert_with(NodeStateStore::default);
                    chain_state.ticket_price = msg.new_value;
                    info!(
                        chain_id = msg.chain_id,
                        old_ticket_price = ?msg.old_value,
                        new_ticket_price = ?msg.new_value,
                        "ConfigurationUpdated - ticket price updated"
                    );
                    Ok(state_map)
                })?;
            }
            Ok(())
        })
    }
}

impl Handler<TypedEvent<CommitteePublished>> for Sortition {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<CommitteePublished>,
        _: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        trap(EType::Sortition, &self.bus.with_ec(&ec), || {
            self.node_state.try_mutate(&ec, |mut state_map| {
                let chain_id = msg.e3_id.chain_id();
                let e3_id_str = format!("{}:{}", chain_id, msg.e3_id.e3_id());
                let chain_state = state_map
                    .entry(chain_id)
                    .or_insert_with(NodeStateStore::default);

                chain_state
                    .e3_committees
                    .insert(e3_id_str.clone(), msg.nodes.clone());

                for node_addr in &msg.nodes {
                    let node = chain_state
                        .nodes
                        .entry(node_addr.clone())
                        .or_insert_with(NodeState::default);
                    node.active_jobs += 1;

                    info!(
                        node = %node_addr,
                        chain_id = chain_id,
                        e3_id = ?msg.e3_id,
                        active_jobs = node.active_jobs,
                        "Incremented active jobs for node in committee"
                    );
                }

                Ok(state_map)
            })
        })
    }
}

impl<T> Handler<E3CommitteeContainsRequest<T>> for Sortition
where
    T: Clone + Send + Sync + 'static,
{
    type Result = ();
    fn handle(
        &mut self,
        msg: E3CommitteeContainsRequest<T>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trap(EType::Sortition, &self.bus.clone(), || {
            let response = E3CommitteeContainsResponse::new(
                msg.inner,
                self.committee_contains(msg.e3_id, msg.node),
            );
            msg.sender.try_send(response)?;
            Ok(())
        })
    }
}

impl Handler<TypedEvent<PlaintextOutputPublished>> for Sortition {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<PlaintextOutputPublished>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        trap(EType::Sortition, &self.bus.with_ec(&ec), || {
            self.decrement_jobs_for_e3(&msg.e3_id, "PlaintextOutputPublished", ec)
        })
    }
}

impl Handler<TypedEvent<E3Failed>> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: TypedEvent<E3Failed>, _ctx: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        trap(EType::Sortition, &self.bus.with_ec(&ec), || {
            let reason = format!("E3Failed: {:?}", msg.reason);
            self.decrement_jobs_for_e3(&msg.e3_id, &reason, ec)
        })
    }
}

impl Handler<TypedEvent<E3StageChanged>> for Sortition {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<E3StageChanged>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        trap(EType::Sortition, &self.bus.with_ec(&ec), || {
            match msg.new_stage {
                E3Stage::Complete | E3Stage::Failed => {
                    let reason = format!("E3StageChanged to {:?}", msg.new_stage);
                    self.decrement_jobs_for_e3(&msg.e3_id, &reason, ec)?;
                }
                _ => {
                    // Non-terminal stages, no action needed
                }
            }
            Ok(())
        })
    }
}

impl Handler<TypedEvent<CommitteeFinalized>> for Sortition {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<CommitteeFinalized>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        trap(EType::Sortition, &self.bus.with_ec(&ec), || {
            info!(
                e3_id = %msg.e3_id,
                committee_size = msg.committee.len(),
                "Storing finalized committee"
            );

            self.finalized_committees.try_mutate(&ec, |mut committees| {
                committees.insert(msg.e3_id.clone(), Committee::new(msg.committee.clone()));
                Ok(committees)
            })
        })
    }
}

impl Handler<TypedEvent<CommitteeMemberExpelled>> for Sortition {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<CommitteeMemberExpelled>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let (data, ec) = msg.into_components();

        // Only process raw events from chain (party_id not yet resolved).
        // Events we re-publish with party_id set will also arrive here; ignore them.
        if data.party_id.is_some() {
            return;
        }

        trap(EType::Sortition, &self.bus.with_ec(&ec), || {
            let node_addr = data.node.to_string();

            let Some(committee) = self.get_committee(&data.e3_id) else {
                warn!(
                    "CommitteeMemberExpelled for node {} but no finalized committee found for e3_id={}. \
                     The committee should always be finalized before expulsions.",
                    node_addr, data.e3_id
                );
                return Ok(());
            };

            let Some(party_id) = committee.party_id_for(&node_addr) else {
                warn!(
                    "Expelled node {} not found in committee for e3_id={}",
                    node_addr, data.e3_id
                );
                return Ok(());
            };

            info!(
                "Sortition: resolved expelled node {} to party_id={} for e3_id={}, re-publishing enriched event",
                node_addr, party_id, data.e3_id
            );

            // Re-publish the event with party_id set to downstream actors
            self.bus.publish(
                CommitteeMemberExpelled {
                    party_id: Some(party_id),
                    ..data
                },
                ec,
            )?;

            Ok(())
        })
    }
}
