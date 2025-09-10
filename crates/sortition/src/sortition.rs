// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::ticket::{best_ticket_for_node, RegisteredNode, WinnerTicket};
use crate::ticket_sortition::ScoreSortition;
use actix::prelude::*;
use alloy::primitives::Address;
use anyhow::{anyhow, Result};
use e3_data::{AutoPersist, Persistable, Repository};
use e3_events::{
    BusError, CiphernodeAdded, CiphernodeRemoved, EnclaveErrorType, EnclaveEvent, EventBus, Seed,
    Subscribe,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, instrument, trace};

/// Message used to query whether a given node (identified by `address`)
/// belongs to the committee of size `size` for a round with seed `seed`
/// in a particular `chain_id`.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "bool")]
pub struct GetHasNode {
    pub seed: Seed,
    pub address: String,
    pub size: usize,
    pub chain_id: u64,
}

/// Abstract list of nodes participating in sortition.
///
/// Implementations should support adding, removing,
/// and checking committee membership.
pub trait SortitionList<T> {
    /// Check if `address` is in the committee of given `size`
    /// under randomness `seed`.
    fn contains(&self, seed: Seed, size: usize, address: T) -> Result<bool>;

    /// Add a new node to the list.
    fn add(&mut self, address: T);

    /// Remove a node from the list.
    fn remove(&mut self, address: T);
}

/// Per-chain registry of registered nodes, each carrying
/// their address and ticket array.
///
/// This replaces the earlier `HashSet<String>` with
/// a richer `RegisteredNode` structure that holds tickets.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SortitionModule {
    registered: Vec<RegisteredNode>,
}

impl SortitionModule {
    /// Construct an empty module (no registered nodes).
    pub fn new() -> Self {
        Self {
            registered: Vec::new(),
        }
    }

    /// Return node addresses (as hex strings).
    /// Useful for diagnostics or UI.
    pub fn nodes(&self) -> Vec<String> {
        self.registered
            .iter()
            .map(|n| n.address.to_string())
            .collect()
    }

    /// Internal helper: find index of a node by address.
    fn find_index(&self, addr: &Address) -> Option<usize> {
        self.registered.iter().position(|n| &n.address == addr)
    }
}

impl Default for SortitionModule {
    fn default() -> Self {
        Self::new()
    }
}

impl SortitionList<String> for SortitionModule {
    fn contains(&self, seed: Seed, size: usize, address: String) -> Result<bool> {
        if self.registered.is_empty() || size == 0 {
            return Ok(false);
        }

        let seed_u64: u64 = seed.into();

        // Compute per-node winners (skip nodes without tickets)
        let mut winners: Vec<WinnerTicket> = Vec::with_capacity(self.registered.len());
        for node in &self.registered {
            if node.tickets.is_empty() {
                continue;
            }
            if let Ok(w) = best_ticket_for_node(seed_u64, node) {
                winners.push(w);
            }
        }

        if winners.is_empty() {
            return Ok(false);
        }

        // Select top-N committee
        let committee = ScoreSortition::new(size).get_committee(&winners)?;

        // Membership check
        let want: Address = address.parse()?;
        Ok(committee.iter().any(|w| w.address == want))
    }

    fn add(&mut self, address: String) {
        if let Ok(addr) = address.parse::<Address>() {
            // Avoid duplicates
            if self.find_index(&addr).is_none() {
                self.registered.push(RegisteredNode {
                    address: addr,
                    tickets: Vec::new(), // start empty; tickets must be populated externally
                });
            }
        }
    }

    fn remove(&mut self, address: String) {
        if let Ok(addr) = address.parse::<Address>() {
            if let Some(i) = self.find_index(&addr) {
                self.registered.swap_remove(i);
            }
        }
    }
}

/// Query message: retrieve the list of nodes registered
/// under a given `chain_id`.
#[derive(Message)]
#[rtype(result = "Vec<String>")]
pub struct GetNodes {
    pub chain_id: u64,
}

/// Actor that manages sortition state across chains.
///
/// Holds a persistent map of `chain_id -> SortitionModule`,
/// and subscribes to enclave events for adding/removing nodes.
pub struct Sortition {
    list: Persistable<HashMap<u64, SortitionModule>>,
    bus: Addr<EventBus<EnclaveEvent>>,
}

/// Parameters required to construct a `Sortition` actor.
#[derive(Debug)]
pub struct SortitionParams {
    pub bus: Addr<EventBus<EnclaveEvent>>,
    pub list: Persistable<HashMap<u64, SortitionModule>>,
}

impl Sortition {
    /// Construct a new `Sortition` actor.
    pub fn new(params: SortitionParams) -> Self {
        Self {
            list: params.list,
            bus: params.bus,
        }
    }

    /// Attach the `Sortition` actor to the event bus.
    ///
    /// Loads the per-chain registry from storage (or default),
    /// starts the actor, and subscribes it to `CiphernodeAdded` events.
    #[instrument(name = "sortition", skip_all)]
    pub async fn attach(
        bus: &Addr<EventBus<EnclaveEvent>>,
        store: Repository<HashMap<u64, SortitionModule>>,
    ) -> Result<Addr<Sortition>> {
        let list = store.load_or_default(HashMap::new()).await?;
        let addr = Sortition::new(SortitionParams {
            bus: bus.clone(),
            list,
        })
        .start();
        bus.do_send(Subscribe::new("CiphernodeAdded", addr.clone().into()));
        Ok(addr)
    }

    /// Retrieve all node addresses registered for a given chain.
    pub fn get_nodes(&self, chain_id: u64) -> Result<Vec<String>> {
        let list_by_chain_id = self.list.get().ok_or(anyhow!(
            "Could not get sortition's list cache. This should not happen."
        ))?;
        let list = list_by_chain_id
            .get(&chain_id)
            .ok_or(anyhow!("No list found for chain_id {}", chain_id))?;
        Ok(list.nodes())
    }
}

impl Actor for Sortition {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for Sortition {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::CiphernodeAdded { data, .. } => ctx.notify(data.clone()),
            EnclaveEvent::CiphernodeRemoved { data, .. } => ctx.notify(data.clone()),
            _ => (),
        }
    }
}

impl Handler<CiphernodeAdded> for Sortition {
    type Result = ();

    /// Handle enclave event: add a node to the registry.
    #[instrument(name = "sortition", skip_all)]
    fn handle(&mut self, msg: CiphernodeAdded, _ctx: &mut Self::Context) -> Self::Result {
        trace!("Adding node: {}", msg.address);
        match self.list.try_mutate(|mut list_map| {
            list_map
                .entry(msg.chain_id)
                .or_insert_with(SortitionModule::default)
                .add(msg.address);
            Ok(list_map)
        }) {
            Err(err) => self.bus.err(EnclaveErrorType::Sortition, err),
            _ => (),
        };
    }
}

impl Handler<CiphernodeRemoved> for Sortition {
    type Result = ();

    /// Handle enclave event: remove a node from the registry.
    #[instrument(name = "sortition", skip_all)]
    fn handle(&mut self, msg: CiphernodeRemoved, _ctx: &mut Self::Context) -> Self::Result {
        info!("Removing node: {}", msg.address);
        match self.list.try_mutate(|mut list_map| {
            list_map
                .get_mut(&msg.chain_id)
                .ok_or(anyhow!(
                    "Cannot remove a node from list that does not exist. It appears that the list for chain_id '{}' has not yet been created.", &msg.chain_id
                ))?
                .remove(msg.address);
            Ok(list_map)
        }) {
            Err(err) => self.bus.err(EnclaveErrorType::Sortition, err),
            _ => (),
        };
    }
}

impl Handler<GetHasNode> for Sortition {
    type Result = bool;

    /// Handle query: check if a node belongs to the current committee.
    #[instrument(name = "sortition", skip_all)]
    fn handle(&mut self, msg: GetHasNode, _ctx: &mut Self::Context) -> Self::Result {
        self.list
            .try_with(|list_map| {
                if let Some(entry) = list_map.get(&msg.chain_id) {
                    return entry.contains(msg.seed, msg.size, msg.address);
                }
                Ok(false)
            })
            .unwrap_or_else(|err| {
                self.bus.err(EnclaveErrorType::Sortition, err);
                false
            })
    }
}

impl Handler<GetNodes> for Sortition {
    type Result = Vec<String>;

    /// Handle query: return all nodes for a given chain ID.
    fn handle(&mut self, msg: GetNodes, _ctx: &mut Self::Context) -> Self::Result {
        self.get_nodes(msg.chain_id).unwrap_or_default()
    }
}
