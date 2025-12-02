// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::backends::{SortitionBackend, SortitionList};
use actix::prelude::*;
use alloy::primitives::U256;
use anyhow::Result;
use e3_data::{AutoPersist, Persistable, Repository};
use e3_events::{
    prelude::*, CiphernodeAdded, CiphernodeRemoved, CommitteeFinalized, CommitteePublished,
    ConfigurationUpdated, EnclaveErrorType, EnclaveEvent, OperatorActivationChanged,
    PlaintextOutputPublished, Seed, TicketBalanceUpdated,
};
use e3_events::{BusHandle, EnclaveEventData};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;
use tracing::instrument;
use tracing::warn;

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

/// Message: ask the `Sortition` whether `address` would be in the
/// committee of size `size` for randomness `seed` on `chain_id`.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "Option<(u64, Option<u64>)>")]
pub struct GetNodeIndex {
    /// Round seed / randomness used by the sortition algorithm.
    pub seed: Seed,
    /// Hex-encoded node address (e.g., `"0x..."`).
    pub address: String,
    /// Committee size (top-N).
    pub size: usize,
    /// Target chain.
    pub chain_id: u64,
}

/// Message: request the current set of registered node addresses for `chain_id`.
#[derive(Message, Clone, Debug)]
#[rtype(result = "Vec<String>")]
pub struct GetNodes {
    /// Target chain.
    pub chain_id: u64,
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
    bus: BusHandle<EnclaveEvent>,
    /// Persistent map of finalized committees per E3
    finalized_committees: Persistable<HashMap<e3_events::E3id, Vec<String>>>,
}

/// Parameters for constructing a `Sortition` actor.
#[derive(Debug)]
pub struct SortitionParams {
    /// Event bus address.
    pub bus: BusHandle<EnclaveEvent>,
    /// Persisted per-chain backend map.
    pub backends: Persistable<HashMap<u64, SortitionBackend>>,
    /// Node state store per chain
    pub node_state: Persistable<HashMap<u64, NodeStateStore>>,
    /// Persistent map of finalized committees per E3
    pub finalized_committees: Persistable<HashMap<e3_events::E3id, Vec<String>>>,
}

impl Sortition {
    pub fn new(params: SortitionParams) -> Self {
        Self {
            backends: params.backends,
            node_state: params.node_state,
            bus: params.bus,
            finalized_committees: params.finalized_committees,
        }
    }

    #[instrument(name = "sortition_attach", skip_all)]
    pub async fn attach(
        bus: &BusHandle<EnclaveEvent>,
        backends_store: Repository<HashMap<u64, SortitionBackend>>,
        node_state_store: Repository<HashMap<u64, NodeStateStore>>,
        committees_store: Repository<HashMap<e3_events::E3id, Vec<String>>>,
        default_backend: SortitionBackend,
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
        })
        .start();

        // Subscribe to all relevant events
        bus.subscribe_all(
            &[
                "CiphernodeAdded",
                "CiphernodeRemoved",
                "TicketBalanceUpdated",
                "OperatorActivationChanged",
                "ConfigurationUpdated",
                "CommitteePublished",
                "PlaintextOutputPublished",
                "CommitteeFinalized",
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
}

impl Actor for Sortition {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg.into_data() {
            EnclaveEventData::CiphernodeAdded(data) => ctx.notify(data.clone()),
            EnclaveEventData::CiphernodeRemoved(data) => ctx.notify(data.clone()),
            EnclaveEventData::TicketBalanceUpdated(data) => ctx.notify(data.clone()),
            EnclaveEventData::OperatorActivationChanged(data) => ctx.notify(data.clone()),
            EnclaveEventData::ConfigurationUpdated(data) => ctx.notify(data.clone()),
            EnclaveEventData::CommitteePublished(data) => ctx.notify(data.clone()),
            EnclaveEventData::PlaintextOutputPublished(data) => ctx.notify(data.clone()),
            EnclaveEventData::CommitteeFinalized(data) => ctx.notify(data.clone()),
            _ => (),
        }
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
            self.bus.err(EnclaveErrorType::Sortition, err);
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
            self.bus.err(EnclaveErrorType::Sortition, err);
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
            self.bus.err(EnclaveErrorType::Sortition, err);
        }

        if let Err(err) = self.backends.try_mutate(move |mut list_map| {
            if let Some(backend) = list_map.get_mut(&chain_id) {
                backend.remove(addr);
            }
            Ok(list_map)
        }) {
            self.bus.err(EnclaveErrorType::Sortition, err);
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
            self.bus.err(EnclaveErrorType::Sortition, err);
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
            self.bus.err(EnclaveErrorType::Sortition, err);
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
                self.bus.err(EnclaveErrorType::Sortition, err);
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
            self.bus.err(EnclaveErrorType::Sortition, err);
        }
    }
}
/// PlaintextOutputPublished is currently used as a signal to decrement the active jobs for the nodes in the committee
/// But in reality, E3 Jobs might not emit that in case there are no votes or the job fails.
/// We need to find a better way to handle the end of an E3, Reduce the jobs in case of of an Error
/// so the tickets do not get locked up.
impl Handler<PlaintextOutputPublished> for Sortition {
    type Result = ();

    fn handle(&mut self, msg: PlaintextOutputPublished, _ctx: &mut Self::Context) -> Self::Result {
        if let Err(err) = self.node_state.try_mutate(|mut state_map| {
            let chain_id = msg.e3_id.chain_id();
            let e3_id_str = format!("{}:{}", chain_id, msg.e3_id.e3_id());

            // Get the committee nodes for this E3
            if let Some(chain_state) = state_map.get_mut(&chain_id) {
                if let Some(committee_nodes) = chain_state.e3_committees.remove(&e3_id_str) {
                    // Decrement active jobs for each node in the committee
                    for node_addr in &committee_nodes {
                        if let Some(node) = chain_state.nodes.get_mut(node_addr) {
                            node.active_jobs = node.active_jobs.saturating_sub(1);

                            info!(
                                node = %node_addr,
                                chain_id = chain_id,
                                e3_id = ?msg.e3_id,
                                active_jobs = node.active_jobs,
                                "Decremented active jobs for node after E3 completion"
                            );
                        }
                    }

                    info!(
                        e3_id = ?msg.e3_id,
                        committee_size = committee_nodes.len(),
                        "PlaintextOutputPublished - job completed, decremented active jobs"
                    );
                } else {
                    info!(
                        e3_id = ?msg.e3_id,
                        "PlaintextOutputPublished - no committee found (might have been completed already)"
                    );
                }
            }

            Ok(state_map)
        }) {
            self.bus.err(EnclaveErrorType::Sortition, err);
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
            self.bus.err(EnclaveErrorType::Sortition, err);
        }
    }
}

impl Handler<GetNodeIndex> for Sortition {
    type Result = ResponseFuture<Option<(u64, Option<u64>)>>;

    fn handle(&mut self, msg: GetNodeIndex, _ctx: &mut Self::Context) -> Self::Result {
        let backends_snapshot = self.backends.get();
        let node_state_snapshot = self.node_state.get();
        let bus = self.bus.clone();

        Box::pin(async move {
            if let (Some(map), Some(state_map)) = (backends_snapshot, node_state_snapshot) {
                if let (Some(backend), Some(state)) =
                    (map.get(&msg.chain_id), state_map.get(&msg.chain_id))
                {
                    backend
                        .get_index(msg.seed, msg.size, msg.address.clone(), msg.chain_id, state)
                        .unwrap_or_else(|err| {
                            bus.err(EnclaveErrorType::Sortition, err);
                            None
                        })
                } else {
                    None
                }
            } else {
                None
            }
        })
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
