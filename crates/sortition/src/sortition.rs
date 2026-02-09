// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::backends::{SortitionBackend, SortitionList};
use crate::ticket_sortition;
use crate::CiphernodeSelector;
use actix::prelude::*;
use alloy::primitives::U256;
use anyhow::Result;
use e3_data::{AutoPersist, Persistable, Repository};
use e3_events::{
    prelude::*, CiphernodeAdded, CiphernodeRemoved, CommitteeFinalized, CommitteePublished,
    ConfigurationUpdated, E3Failed, E3Requested, E3Stage, E3StageChanged, EType, EnclaveEvent,
    EventType, OperatorActivationChanged, PlaintextOutputPublished, Seed, TicketBalanceUpdated,
};
use e3_events::{BusHandle, E3id, EnclaveEventData};
use e3_utils::NotifySync;
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

/// Message: request the current set of registered node addresses for `chain_id`.
#[derive(Message, Clone, Debug)]
#[rtype(result = "Vec<String>")]
pub struct GetNodes {
    /// Target chain.
    pub chain_id: u64,
}

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct WithSortitionPartyTicket<T> {
    inner: T,
    party_ticket_id: Option<(u64, Option<u64>)>,
    address: String,
}

impl<T> WithSortitionPartyTicket<T> {
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

impl<T> Deref for WithSortitionPartyTicket<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Message to get the finalized committee nodes for a specific E3.
#[derive(Message, Clone, Debug)]
#[rtype(result = "Vec<String>")]
pub struct GetNodesForE3 {
    /// E3 ID to get nodes for.
    pub e3_id: e3_events::E3id,
    /// Chain ID
    pub chain_id: u64,
}

/// Message to get the current node state.
#[derive(Message, Clone, Debug)]
#[rtype(result = "Option<HashMap<u64, NodeStateStore>>")]
pub struct GetNodeState;

/// Sortition actor that manages the sortition algorithm and the node state.
pub struct Sortition {
    /// Persistent map of `chain_id -> SortitionBackend`.
    backends: Persistable<HashMap<u64, SortitionBackend>>,
    /// Persistent map of `chain_id -> NodeStateStore`.
    node_state: Persistable<HashMap<u64, NodeStateStore>>,
    /// Event bus for error reporting and enclave event subscription.
    bus: BusHandle,
    /// Persistent map of finalized committees per E3
    finalized_committees: Persistable<HashMap<e3_events::E3id, Vec<String>>>,
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
    pub finalized_committees: Persistable<HashMap<e3_events::E3id, Vec<String>>>,
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
        committees_store: Repository<HashMap<e3_events::E3id, Vec<String>>>,
        default_backend: SortitionBackend,
        ciphernode_selector: Addr<CiphernodeSelector>,
        address: &str,
    ) -> Result<Addr<Self>> {
        let mut backends = backends_store.load_or_default(HashMap::new()).await?;
        let node_state = node_state_store.load_or_default(HashMap::new()).await?;
        let finalized_committees = committees_store.load_or_default(HashMap::new()).await?;

        backends.try_mutate(|mut list| {
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
            .ok_or_else(|| anyhow::anyhow!("Could not get backends cache"))?;
        let backend = map
            .get(&chain_id)
            .ok_or_else(|| anyhow::anyhow!("No backend for chain_id {}", chain_id))?;
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

    /// Helper method to decrement active jobs for an E3's committee
    fn decrement_jobs_for_e3(&mut self, e3_id: &E3id, reason: &str) {
        if let Err(err) = self.node_state.try_mutate(|mut state_map| {
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
        }) {
            self.bus.err(EType::Sortition, err);
        }
    }
}

impl Actor for Sortition {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg.into_data() {
            EnclaveEventData::E3Requested(data) => self.notify_sync(ctx, data.clone()),
            EnclaveEventData::CiphernodeAdded(data) => self.notify_sync(ctx, data.clone()),
            EnclaveEventData::CiphernodeRemoved(data) => self.notify_sync(ctx, data.clone()),
            EnclaveEventData::TicketBalanceUpdated(data) => self.notify_sync(ctx, data.clone()),
            EnclaveEventData::OperatorActivationChanged(data) => {
                self.notify_sync(ctx, data.clone())
            }
            EnclaveEventData::ConfigurationUpdated(data) => self.notify_sync(ctx, data.clone()),
            EnclaveEventData::CommitteePublished(data) => self.notify_sync(ctx, data.clone()),
            EnclaveEventData::PlaintextOutputPublished(data) => self.notify_sync(ctx, data.clone()),
            EnclaveEventData::CommitteeFinalized(data) => self.notify_sync(ctx, data.clone()),
            _ => (),
        }
    }
}

impl Handler<E3Requested> for Sortition {
    type Result = ();
    fn handle(&mut self, msg: E3Requested, _ctx: &mut Self::Context) -> Self::Result {
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

        self.ciphernode_selector
            .do_send(WithSortitionPartyTicket::new(
                msg,
                self.get_node_index(e3_id, seed, total_selection_size, chain_id),
                &self.address,
            ))
    }
}

impl Handler<CiphernodeAdded> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: CiphernodeAdded, _ctx: &mut Self::Context) -> Self::Result {
        let chain_id = msg.chain_id;
        let addr = msg.address.clone();

        if let Err(err) = self.node_state.try_mutate(|mut state_map| {
            let chain_state = state_map
                .entry(chain_id)
                .or_insert_with(NodeStateStore::default);
            chain_state
                .nodes
                .entry(addr.clone())
                .or_insert_with(NodeState::default);
            Ok(state_map)
        }) {
            self.bus.err(EType::Sortition, err);
        }

        if let Err(err) = self.backends.try_mutate(move |mut list_map| {
            let default_backend = list_map
                .get(&u64::MAX)
                .cloned()
                .unwrap_or_else(|| SortitionBackend::score());

            list_map
                .entry(chain_id)
                .or_insert_with(|| default_backend)
                .add(addr);
            Ok(list_map)
        }) {
            self.bus.err(EType::Sortition, err);
        }

        info!(address = %msg.address, chain_id = chain_id, "Node added to sortition state");
    }
}

impl Handler<CiphernodeRemoved> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: CiphernodeRemoved, _ctx: &mut Self::Context) -> Self::Result {
        let chain_id = msg.chain_id;
        let addr = msg.address.clone();

        if let Err(err) = self.node_state.try_mutate(|mut state_map| {
            if let Some(chain_state) = state_map.get_mut(&chain_id) {
                chain_state.nodes.remove(&addr);
            }
            Ok(state_map)
        }) {
            self.bus.err(EType::Sortition, err);
        }

        if let Err(err) = self.backends.try_mutate(move |mut list_map| {
            if let Some(backend) = list_map.get_mut(&chain_id) {
                backend.remove(addr);
            }
            Ok(list_map)
        }) {
            self.bus.err(EType::Sortition, err);
        }

        info!(address = %msg.address, chain_id = chain_id, "Node removed from sortition state");
    }
}

impl Handler<TicketBalanceUpdated> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: TicketBalanceUpdated, _ctx: &mut Self::Context) -> Self::Result {
        if let Err(err) = self.node_state.try_mutate(|mut state_map| {
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
        }) {
            self.bus.err(EType::Sortition, err);
        }
    }
}

impl Handler<OperatorActivationChanged> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: OperatorActivationChanged, _ctx: &mut Self::Context) -> Self::Result {
        if let Err(err) = self.node_state.try_mutate(|mut state_map| {
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
        }) {
            self.bus.err(EType::Sortition, err);
        }
    }
}

impl Handler<ConfigurationUpdated> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: ConfigurationUpdated, _ctx: &mut Self::Context) -> Self::Result {
        if msg.parameter == "ticketPrice" {
            if let Err(err) = self.node_state.try_mutate(|mut state_map| {
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
            }) {
                self.bus.err(EType::Sortition, err);
            }
        }
    }
}

impl Handler<CommitteePublished> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: CommitteePublished, _ctx: &mut Self::Context) -> Self::Result {
        if let Err(err) = self.node_state.try_mutate(|mut state_map| {
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
        }) {
            self.bus.err(EType::Sortition, err);
        }
    }
}

impl Handler<PlaintextOutputPublished> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: PlaintextOutputPublished, _ctx: &mut Self::Context) -> Self::Result {
        self.decrement_jobs_for_e3(&msg.e3_id, "PlaintextOutputPublished");
    }
}

impl Handler<E3Failed> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: E3Failed, _ctx: &mut Self::Context) -> Self::Result {
        let reason = format!("E3Failed: {:?}", msg.reason);
        self.decrement_jobs_for_e3(&msg.e3_id, &reason);
    }
}

impl Handler<E3StageChanged> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: E3StageChanged, _ctx: &mut Self::Context) -> Self::Result {
        match msg.new_stage {
            E3Stage::Complete | E3Stage::Failed => {
                let reason = format!("E3StageChanged to {:?}", msg.new_stage);
                self.decrement_jobs_for_e3(&msg.e3_id, &reason);
            }
            _ => {
                // Non-terminal stages, no action needed
            }
        }
    }
}

impl Handler<CommitteeFinalized> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: CommitteeFinalized, _ctx: &mut Self::Context) -> Self::Result {
        info!(
            e3_id = %msg.e3_id,
            committee_size = msg.committee.len(),
            "Storing finalized committee"
        );

        if let Err(err) = self.finalized_committees.try_mutate(|mut committees| {
            committees.insert(msg.e3_id.clone(), msg.committee.clone());
            Ok(committees)
        }) {
            self.bus.err(EType::Sortition, err);
        }
    }
}

impl Handler<GetNodes> for Sortition {
    type Result = Vec<String>;

    fn handle(&mut self, msg: GetNodes, _ctx: &mut Self::Context) -> Self::Result {
        self.get_nodes(msg.chain_id).unwrap_or_else(|err| {
            tracing::warn!("Failed to get nodes for chain {}: {}", msg.chain_id, err);
            Vec::new()
        })
    }
}

impl Handler<GetNodesForE3> for Sortition {
    type Result = Vec<String>;

    fn handle(&mut self, msg: GetNodesForE3, _ctx: &mut Self::Context) -> Self::Result {
        if msg.e3_id.chain_id() != msg.chain_id {
            tracing::warn!(
                "Chain ID mismatch: e3_id has chain_id {}, but requested chain_id {}",
                msg.e3_id.chain_id(),
                msg.chain_id
            );
            return Vec::new();
        }

        self.finalized_committees
            .get()
            .and_then(|committees| committees.get(&msg.e3_id).cloned())
            .unwrap_or_else(|| {
                tracing::warn!("No finalized committee found for E3 {}", msg.e3_id);
                Vec::new()
            })
    }
}

impl Handler<GetNodeState> for Sortition {
    type Result = Option<HashMap<u64, NodeStateStore>>;

    fn handle(&mut self, _msg: GetNodeState, _: &mut Self::Context) -> Self::Result {
        self.node_state.get()
    }
}
