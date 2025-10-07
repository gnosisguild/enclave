// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::distance::DistanceSortition;
use crate::ticket::{RegisteredNode, Ticket};
use crate::ticket_sortition::ScoreSortition;
use actix::prelude::*;
use alloy::primitives::Address;
use anyhow::{anyhow, bail, Result};
use e3_data::{AutoPersist, Persistable, Repository};
use e3_events::{
    BusError, CiphernodeAdded, CiphernodeRemoved, EnclaveErrorType, EnclaveEvent, EventBus, Seed,
    Subscribe,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{info, instrument, trace};

/// Ask the `Sortition` actor whether `address` is in the committee
/// of size `size` for randomness `seed` on the given `chain_id`.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "bool")]
pub struct GetHasNode {
    /// Round seed / randomness used by the sortition algorithm.
    pub seed: Seed,
    /// Hex-encoded node address (e.g., "0x...").
    pub address: String,
    /// Committee size to consider (top-N).
    pub size: usize,
    /// Chain for which to check membership.
    pub chain_id: u64,
}

/// Ask the `Sortition` actor for the current set of registered node
/// addresses for a `chain_id`.
#[derive(Message, Clone, Debug)]
#[rtype(result = "Vec<String>")]
pub struct GetNodes {
    /// Chain identifier.
    pub chain_id: u64,
}

/// Minimal interface a sortition backend must implement. Backends can
/// store their data however they like (e.g., simple address sets for
/// distance sortition or richer `RegisteredNode` values for score sortition).
pub trait SortitionList<T> {
    /// Return `true` if `address` is in the size-`size` committee under `seed`.
    fn contains(&self, seed: Seed, size: usize, address: T) -> Result<bool>;
    /// Add a node to the backend.
    fn add(&mut self, address: T);
    /// Remove a node from the backend.
    fn remove(&mut self, address: T);
    /// Return all node addresses (hex) for diagnostics / UI.
    fn nodes(&self) -> Vec<String>;
}

/// Backend for *distance* sortition:
/// stores a set of hex string addresses and delegates committee selection
/// to `DistanceSortition`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistanceBackend {
    /// Registered node addresses as hex strings.
    nodes: HashSet<String>,
}

impl Default for DistanceBackend {
    fn default() -> Self {
        Self {
            nodes: HashSet::new(),
        }
    }
}

impl SortitionList<String> for DistanceBackend {
    /// Build the address list, run `DistanceSortition`, and check membership.
    fn contains(&self, seed: Seed, size: usize, address: String) -> Result<bool> {
        if self.nodes.is_empty() || size == 0 {
            return Ok(false);
        }

        let registered_nodes: Vec<Address> = self
            .nodes
            .iter()
            .cloned()
            .map(|s| s.parse::<Address>())
            .collect::<Result<_, _>>()?;

        let committee =
            DistanceSortition::new(seed.into(), registered_nodes, size).get_committee()?;

        Ok(committee
            .iter()
            .any(|(_, addr)| addr.to_string() == address))
    }

    /// Insert a node address (hex string). Idempotent for duplicates.
    fn add(&mut self, address: String) {
        self.nodes.insert(address);
    }

    /// Remove a node address (hex string).
    fn remove(&mut self, address: String) {
        self.nodes.remove(&address);
    }

    /// Return all node addresses as hex strings.
    fn nodes(&self) -> Vec<String> {
        self.nodes.iter().cloned().collect()
    }
}

/// Backend for *score* sortition:
/// stores `RegisteredNode` entries (address + tickets), and enforces
/// global ticket_id uniqueness via `used_ticket_ids`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoreBackend {
    /// Nodes with their ticket sets (used by score-based committee selection).
    registered: Vec<RegisteredNode>,
    /// Guardrail: prevents duplicate `ticket_id`s across the whole backend.
    used_ticket_ids: HashSet<u64>,
}

impl Default for ScoreBackend {
    fn default() -> Self {
        Self {
            registered: Vec::new(),
            used_ticket_ids: HashSet::new(),
        }
    }
}

impl ScoreBackend {
    /// Add or replace the tickets for a given `address` (as an `Address` type).
    ///
    /// This method:
    /// - Sorts and deduplicates incoming `tickets` by `ticket_id`.
    /// - If the node already exists, frees its current `ticket_id`s from
    ///   `used_ticket_ids` and then inserts the new set.
    /// - If the node does not exist, creates it with the provided tickets.
    /// - Enforces global uniqueness of `ticket_id`s; returns an error if any
    ///   incoming `ticket_id` already exists elsewhere.
    pub fn add_with_tickets_addr(
        &mut self,
        address: Address,
        tickets: Option<Vec<Ticket>>,
    ) -> Result<()> {
        let mut tickets = tickets.unwrap_or_default();
        tickets.sort_unstable_by_key(|t| t.ticket_id);
        tickets.dedup_by_key(|t| t.ticket_id);

        // Node exists: reclaim its ticket ids, then set the new tickets.
        if let Some(existing) = self.registered.iter_mut().find(|n| n.address == address) {
            for ticket in &existing.tickets {
                self.used_ticket_ids.remove(&ticket.ticket_id);
            }
            for ticket in &tickets {
                if !self.used_ticket_ids.insert(ticket.ticket_id) {
                    bail!("duplicate ticket id detected: {}", ticket.ticket_id);
                }
            }
            existing.tickets = tickets;
            return Ok(());
        }

        // New node: just enforce uniqueness and push.
        for ticket in &tickets {
            if !self.used_ticket_ids.insert(ticket.ticket_id) {
                bail!("duplicate ticket id detected: {}", ticket.ticket_id);
            }
        }

        self.registered.push(RegisteredNode { address, tickets });
        Ok(())
    }
}

impl SortitionList<String> for ScoreBackend {
    /// Compute score-based winners and check if `address` is included.
    fn contains(&self, seed: Seed, size: usize, address: String) -> Result<bool> {
        if self.registered.is_empty() || size == 0 {
            return Ok(false);
        }

        let winners = ScoreSortition::new(size).get_committee(seed.into(), &self.registered)?;
        let want: Address = address.parse()?;
        Ok(winners.iter().any(|w| w.address == want))
    }

    /// Add a node with an empty ticket set if it doesn't exist.
    /// (Use `add_with_tickets_addr` to set tickets explicitly.)
    fn add(&mut self, address: String) {
        match address.parse::<Address>() {
            Ok(addr) => {
                if !self.registered.iter().any(|n| n.address == addr) {
                    self.registered.push(RegisteredNode {
                        address: addr,
                        tickets: Vec::new(),
                    });
                }
            }
            Err(e) => {
                tracing::warn!("Failed to parse address '{}': {}", address, e);
            }
        }
    }

    /// Remove the node and free its `ticket_id`s from the global guard.
    fn remove(&mut self, address: String) {
        if let Ok(addr) = address.parse::<Address>() {
            if let Some(i) = self.registered.iter().position(|n| n.address == addr) {
                for t in &self.registered[i].tickets {
                    self.used_ticket_ids.remove(&t.ticket_id);
                }
                self.registered.swap_remove(i);
            }
        }
    }

    /// Return all registered node addresses as hex strings.
    fn nodes(&self) -> Vec<String> {
        self.registered
            .iter()
            .map(|n| n.address.to_string())
            .collect()
    }
}

/// An enum wrapper around the two supported backends.
/// New chains should default to `Distance`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SortitionBackend {
    /// Distance-based selection (stores a simple set of addresses).
    Distance(DistanceBackend),
    /// Score-based selection (stores `RegisteredNode`s with tickets).
    Score(ScoreBackend),
}

impl SortitionBackend {
    /// Construct a `SortitionBackend` preconfigured with a default `DistanceBackend`.
    pub fn default_distance() -> Self {
        SortitionBackend::Distance(DistanceBackend::default())
    }

    /// Convenience: for Score backends, add/replace tickets for a node by `Address`.
    /// Returns an error if the backend is `Distance`.
    pub fn add_with_tickets_addr(
        &mut self,
        address: Address,
        tickets: Option<Vec<Ticket>>,
    ) -> Result<()> {
        match self {
            SortitionBackend::Score(b) => b.add_with_tickets_addr(address, tickets),
            SortitionBackend::Distance(_) => {
                bail!("add_with_tickets is only valid for Score backend")
            }
        }
    }
}

impl SortitionList<String> for SortitionBackend {
    fn contains(&self, seed: Seed, size: usize, address: String) -> Result<bool> {
        match self {
            SortitionBackend::Distance(backend) => backend.contains(seed, size, address),
            SortitionBackend::Score(backend) => backend.contains(seed, size, address),
        }
    }

    fn add(&mut self, address: String) {
        match self {
            SortitionBackend::Distance(backend) => backend.add(address),
            SortitionBackend::Score(backend) => backend.add(address),
        }
    }

    fn remove(&mut self, address: String) {
        match self {
            SortitionBackend::Distance(backend) => backend.remove(address),
            SortitionBackend::Score(backend) => backend.remove(address),
        }
    }

    fn nodes(&self) -> Vec<String> {
        match self {
            SortitionBackend::Distance(backend) => backend.nodes(),
            SortitionBackend::Score(backend) => backend.nodes(),
        }
    }
}

/// `Sortition` is an Actix actor that owns the per-chain backends and
/// exposes message handlers to add/remove nodes, list nodes, and check
/// committee membership.
///
/// Persistence is handled via `Persistable<HashMap<u64, SortitionBackend>>`,
/// keyed by `chain_id`.
pub struct Sortition {
    /// Persistent map of `chain_id -> SortitionBackend`.
    list: Persistable<HashMap<u64, SortitionBackend>>,
    /// Event bus for error reporting and enclave event subscription.
    bus: Addr<EventBus<EnclaveEvent>>,
}

/// Parameters for constructing a `Sortition` actor.
#[derive(Debug)]
pub struct SortitionParams {
    /// Event bus address.
    pub bus: Addr<EventBus<EnclaveEvent>>,
    /// Persisted per-chain backend map.
    pub list: Persistable<HashMap<u64, SortitionBackend>>,
}

impl Sortition {
    /// Construct a new `Sortition` actor with the given bus and repository.
    pub fn new(params: SortitionParams) -> Self {
        Self {
            list: params.list,
            bus: params.bus,
        }
    }

    /// Load persisted state, start the actor, and subscribe to `CiphernodeAdded/Removed`.
    #[instrument(name = "sortition_attach", skip_all)]
    pub async fn attach(
        bus: &Addr<EventBus<EnclaveEvent>>,
        store: Repository<HashMap<u64, SortitionBackend>>,
    ) -> Result<Addr<Sortition>> {
        let list = store.load_or_default(HashMap::new()).await?;
        let addr = Sortition::new(SortitionParams {
            bus: bus.clone(),
            list,
        })
        .start();
        bus.do_send(Subscribe::new("CiphernodeAdded", addr.clone().into()));
        bus.do_send(Subscribe::new("CiphernodeRemoved", addr.clone().into()));
        Ok(addr)
    }

    /// Return the current node addresses (hex) for `chain_id`.
    pub fn get_nodes(&self, chain_id: u64) -> Result<Vec<String>> {
        let map = self
            .list
            .get()
            .ok_or_else(|| anyhow!("Could not get sortition's list cache"))?;
        let backend = map
            .get(&chain_id)
            .ok_or_else(|| anyhow!("No list for chain_id {}", chain_id))?;
        Ok(backend.nodes())
    }
}

impl Actor for Sortition {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for Sortition {
    type Result = ();
    /// Fan-in enclave events to the corresponding typed handlers.
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

    /// Add a node to the target chain. If the chain does not exist yet,
    /// initialize it with the default `Distance` backend.
    #[instrument(name = "sortition_add_node", skip_all)]
    fn handle(&mut self, msg: CiphernodeAdded, _ctx: &mut Self::Context) -> Self::Result {
        trace!("Adding node: {}", msg.address);
        let chain_id = msg.chain_id;
        let addr = msg.address.clone();

        if let Err(err) = self.list.try_mutate(move |mut list_map| {
            list_map
                .entry(chain_id)
                .or_insert_with(SortitionBackend::default_distance)
                .add(addr);
            Ok(list_map)
        }) {
            self.bus.err(EnclaveErrorType::Sortition, err);
        }
    }
}

impl Handler<CiphernodeRemoved> for Sortition {
    type Result = ();

    /// Remove a node from the target chain, initializing the chain entry
    /// with a default `Distance` backend if it was missing.
    #[instrument(name = "sortition_remove_node", skip_all)]
    fn handle(&mut self, msg: CiphernodeRemoved, _ctx: &mut Self::Context) -> Self::Result {
        info!("Removing node: {}", msg.address);
        let chain_id = msg.chain_id;
        let addr = msg.address.clone();

        if let Err(err) = self.list.try_mutate(move |mut list_map| {
            if let Some(backend) = list_map.get_mut(&chain_id) {
                backend.remove(addr);
            }
            Ok(list_map)
        }) {
            self.bus.err(EnclaveErrorType::Sortition, err);
        }
    }
}

impl Handler<GetHasNode> for Sortition {
    type Result = bool;

    /// Return whether `address` is in the size-`size` committee for `seed`
    /// on `chain_id`. If the chain has not been initialized, returns `false`.
    #[instrument(name = "sortition_contains", skip_all)]
    fn handle(&mut self, msg: GetHasNode, _ctx: &mut Self::Context) -> Self::Result {
        self.list
            .try_with(|map| {
                if let Some(backend) = map.get(&msg.chain_id) {
                    backend.contains(msg.seed, msg.size, msg.address.clone())
                } else {
                    Ok(false)
                }
            })
            .unwrap_or_else(|err| {
                self.bus.err(EnclaveErrorType::Sortition, err);
                false
            })
    }
}

impl Handler<GetNodes> for Sortition {
    type Result = Vec<String>;

    /// Return all registered node addresses for a chain, or `[]` on error.
    fn handle(&mut self, msg: GetNodes, _ctx: &mut Self::Context) -> Self::Result {
        self.get_nodes(msg.chain_id).unwrap_or_else(|err| {
            tracing::warn!("Failed to get nodes for chain {}: {}", msg.chain_id, err);
            Vec::new()
        })
    }
}
