// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use alloy::primitives::U256;
use anyhow::Result;
use e3_data::{AutoPersist, Persistable, Repository};
use e3_events::{
    BusError, CiphernodeAdded, CiphernodeRemoved, CommitteePublished, ConfigurationUpdated,
    EnclaveErrorType, EnclaveEvent, EventBus, OperatorActivationChanged, PlaintextOutputPublished,
    Subscribe, TicketBalanceUpdated,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

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

/// State for all nodes across all chains
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NodeStateStore {
    /// Map of (chain_id, node_address) to node state
    pub nodes: HashMap<(u64, String), NodeState>,
    /// Current ticket price per chain
    pub ticket_prices: HashMap<u64, U256>,
    /// Map of E3 ID to the committee nodes for that E3
    /// This is used to track which nodes are in which E3 jobs
    pub e3_committees: HashMap<String, Vec<String>>,
}

impl NodeStateStore {
    /// Get available tickets for a node, accounting for active jobs
    pub fn available_tickets(&self, chain_id: u64, address: &str) -> u64 {
        let ticket_price = self
            .ticket_prices
            .get(&chain_id)
            .copied()
            .unwrap_or(U256::from(1));
        if ticket_price.is_zero() {
            return 0;
        }

        let key = (chain_id, address.to_string());
        let node = self.nodes.get(&key);

        if let Some(node) = node {
            let total_tickets = (node.ticket_balance / ticket_price)
                .try_into()
                .unwrap_or(0u64);
            // Subtract active jobs from available tickets
            total_tickets.saturating_sub(node.active_jobs)
        } else {
            0
        }
    }

    /// Get all nodes for a chain with their available tickets
    /// Only includes active nodes
    pub fn get_nodes_with_tickets(&self, chain_id: u64) -> Vec<(String, u64)> {
        self.nodes
            .iter()
            .filter(|((cid, _), node_state)| *cid == chain_id && node_state.active)
            .map(|((_, addr), _)| (addr.clone(), self.available_tickets(chain_id, addr)))
            .filter(|(_, tickets)| *tickets > 0)
            .collect()
    }
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "Option<NodeStateStore>")]
pub struct GetNodeState;

pub struct NodeStateManager {
    state: Persistable<NodeStateStore>,
    bus: Addr<EventBus<EnclaveEvent>>,
}

impl NodeStateManager {
    pub fn new(state: Persistable<NodeStateStore>, bus: Addr<EventBus<EnclaveEvent>>) -> Self {
        Self { state, bus }
    }

    pub async fn attach(
        bus: &Addr<EventBus<EnclaveEvent>>,
        repository: &Repository<NodeStateStore>,
    ) -> Result<Addr<Self>> {
        let state = repository
            .clone()
            .load_or_default(NodeStateStore::default())
            .await?;

        let addr = NodeStateManager::new(state, bus.clone()).start();

        bus.send(Subscribe::new("CiphernodeAdded", addr.clone().into()))
            .await?;
        bus.send(Subscribe::new("CiphernodeRemoved", addr.clone().into()))
            .await?;
        bus.send(Subscribe::new("TicketBalanceUpdated", addr.clone().into()))
            .await?;
        bus.send(Subscribe::new(
            "OperatorActivationChanged",
            addr.clone().into(),
        ))
        .await?;
        bus.send(Subscribe::new("ConfigurationUpdated", addr.clone().into()))
            .await?;
        bus.send(Subscribe::new("CommitteePublished", addr.clone().into()))
            .await?;
        bus.send(Subscribe::new(
            "PlaintextOutputPublished",
            addr.clone().into(),
        ))
        .await?;

        info!("NodeStateManager actor started");
        Ok(addr)
    }
}

impl Actor for NodeStateManager {
    type Context = Context<Self>;
}

impl Handler<GetNodeState> for NodeStateManager {
    type Result = Option<NodeStateStore>;

    fn handle(&mut self, _msg: GetNodeState, _: &mut Self::Context) -> Self::Result {
        self.state.get()
    }
}

impl Handler<EnclaveEvent> for NodeStateManager {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::CiphernodeAdded { data, .. } => {
                ctx.notify(data);
            }
            EnclaveEvent::CiphernodeRemoved { data, .. } => {
                ctx.notify(data);
            }
            EnclaveEvent::TicketBalanceUpdated { data, .. } => {
                ctx.notify(data);
            }
            EnclaveEvent::OperatorActivationChanged { data, .. } => {
                ctx.notify(data);
            }
            EnclaveEvent::ConfigurationUpdated { data, .. } => {
                ctx.notify(data);
            }
            EnclaveEvent::CommitteePublished { data, .. } => {
                ctx.notify(data);
            }
            EnclaveEvent::PlaintextOutputPublished { data, .. } => {
                ctx.notify(data);
            }
            _ => (),
        }
    }
}

impl Handler<TicketBalanceUpdated> for NodeStateManager {
    type Result = ();

    fn handle(&mut self, msg: TicketBalanceUpdated, _: &mut Self::Context) -> Self::Result {
        match self.state.try_mutate(|mut state| {
            let key = (msg.chain_id, msg.operator.clone());
            let node = state.nodes.entry(key).or_insert_with(NodeState::default);

            // Update ticket balance
            node.ticket_balance = msg.new_balance;

            info!(
                operator = %msg.operator,
                chain_id = msg.chain_id,
                new_balance = ?msg.new_balance,
                "Updated ticket balance"
            );

            Ok(state)
        }) {
            Ok(_) => (),
            Err(err) => self.bus.err(EnclaveErrorType::Sortition, err),
        }
    }
}

impl Handler<OperatorActivationChanged> for NodeStateManager {
    type Result = ();

    fn handle(&mut self, msg: OperatorActivationChanged, _: &mut Self::Context) -> Self::Result {
        match self.state.try_mutate(|mut state| {
            // We don't have chain_id in this event, so we need to update all entries for this operator
            // In practice, an operator should only be registered on one chain, but we handle all just in case
            for ((_, addr), node) in state.nodes.iter_mut() {
                if addr == &msg.operator {
                    node.active = msg.active;
                    info!(
                        operator = %msg.operator,
                        active = msg.active,
                        "Updated operator active status"
                    );
                }
            }
            Ok(state)
        }) {
            Ok(_) => (),
            Err(err) => self.bus.err(EnclaveErrorType::Sortition, err),
        }
    }
}

impl Handler<ConfigurationUpdated> for NodeStateManager {
    type Result = ();

    fn handle(&mut self, msg: ConfigurationUpdated, _: &mut Self::Context) -> Self::Result {
        match self.state.try_mutate(|mut state| {
            if msg.parameter == "ticketPrice" {
                state.ticket_prices.insert(msg.chain_id, msg.new_value);
                info!(
                    chain_id = msg.chain_id,
                    old_ticket_price = ?msg.old_value,
                    new_ticket_price = ?msg.new_value,
                    "ConfigurationUpdated - ticket price updated"
                );
            }
            Ok(state)
        }) {
            Ok(_) => (),
            Err(err) => self.bus.err(EnclaveErrorType::Sortition, err),
        }
    }
}

impl Handler<CommitteePublished> for NodeStateManager {
    type Result = ();

    fn handle(&mut self, msg: CommitteePublished, _: &mut Self::Context) -> Self::Result {
        match self.state.try_mutate(|mut state| {
            let chain_id = msg.e3_id.chain_id();
            let e3_id_str = format!("{}:{}", chain_id, msg.e3_id.e3_id());

            // Store the committee mapping for this E3
            state
                .e3_committees
                .insert(e3_id_str.clone(), msg.nodes.clone());

            // Increment active jobs for each node in the committee
            for node_addr in &msg.nodes {
                let key = (chain_id, node_addr.clone());
                let node = state.nodes.entry(key).or_insert_with(NodeState::default);
                node.active_jobs += 1;

                info!(
                    node = %node_addr,
                    chain_id = chain_id,
                    e3_id = ?msg.e3_id,
                    active_jobs = node.active_jobs,
                    "Incremented active jobs for node in committee"
                );
            }

            Ok(state)
        }) {
            Ok(_) => (),
            Err(err) => self.bus.err(EnclaveErrorType::Sortition, err),
        }
    }
}

impl Handler<PlaintextOutputPublished> for NodeStateManager {
    type Result = ();

    fn handle(&mut self, msg: PlaintextOutputPublished, _: &mut Self::Context) -> Self::Result {
        match self.state.try_mutate(|mut state| {
            let chain_id = msg.e3_id.chain_id();
            let e3_id_str = format!("{}:{}", chain_id, msg.e3_id.e3_id());

            // Get the committee nodes for this E3
            if let Some(committee_nodes) = state.e3_committees.remove(&e3_id_str) {
                // Decrement active jobs for each node in the committee
                for node_addr in &committee_nodes {
                    let key = (chain_id, node_addr.clone());
                    if let Some(node) = state.nodes.get_mut(&key) {
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

            Ok(state)
        }) {
            Ok(_) => (),
            Err(err) => self.bus.err(EnclaveErrorType::Sortition, err),
        }
    }
}

impl Handler<CiphernodeAdded> for NodeStateManager {
    type Result = ();

    fn handle(&mut self, msg: CiphernodeAdded, _: &mut Self::Context) -> Self::Result {
        match self.state.try_mutate(|mut state| {
            let key = (msg.chain_id, msg.address.clone());
            // Only create entry if it doesn't exist - preserve existing state
            state.nodes.entry(key).or_insert_with(NodeState::default);

            info!(
                operator = %msg.address,
                chain_id = msg.chain_id,
                "Node registered in NodeStateManager"
            );

            Ok(state)
        }) {
            Ok(_) => (),
            Err(err) => self.bus.err(EnclaveErrorType::Sortition, err),
        }
    }
}

impl Handler<CiphernodeRemoved> for NodeStateManager {
    type Result = ();

    fn handle(&mut self, msg: CiphernodeRemoved, _: &mut Self::Context) -> Self::Result {
        match self.state.try_mutate(|mut state| {
            let key = (msg.chain_id, msg.address.clone());
            state.nodes.remove(&key);

            info!(
                operator = %msg.address,
                chain_id = msg.chain_id,
                "Node removed from NodeStateManager"
            );

            Ok(state)
        }) {
            Ok(_) => (),
            Err(err) => self.bus.err(EnclaveErrorType::Sortition, err),
        }
    }
}
