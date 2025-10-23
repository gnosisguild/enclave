// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::distance::DistanceSortition;
use crate::node_state::NodeStateStore;
use crate::ticket::{RegisteredNode, Ticket};
use crate::ticket_sortition::ScoreSortition;
use actix::prelude::*;
use alloy::primitives::Address;
use anyhow::{anyhow, Context, Result};
use e3_data::{AutoPersist, Persistable, Repository};
use e3_events::{
    BusError, CiphernodeAdded, CiphernodeRemoved, EnclaveErrorType, EnclaveEvent, EventBus, Seed,
    Subscribe,
};
use num::BigInt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{info, instrument, trace};

/// Message: ask the `Sortition` actor whether `address` would be in the
/// committee of size `size` for randomness `seed` on `chain_id`.
///
/// Membership semantics depend on the backend for that chain:
/// - **Distance backend**: computes a committee using address distance.
/// - **Score backend**: computes each node’s best ticket score and sorts globally.
/// Returns `true` if `address` appears in the resulting top-`size` selection.
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

/// Minimal interface that all sortition backends must implement.
///
/// Backends can store their own shapes (e.g., a `HashSet<String>` of addresses
/// for Distance, or a `Vec<RegisteredNode>` for Score), but they must be able to:
/// - Check committee membership (`contains`)
/// - Add and remove nodes
/// - List all registered node addresses
pub trait SortitionList<T> {
    /// Return `true` if `address` appears in the size-`size` committee under `seed`.
    ///
    /// Implementations should return `Ok(false)` if the backend has no nodes
    /// or if `size == 0`.
    fn contains(
        &self,
        seed: Seed,
        size: usize,
        address: T,
        node_state: Option<&NodeStateStore>,
        chain_id: u64,
    ) -> anyhow::Result<bool>;

    /// Return an index if `address` appears in the committee under `seed`.
    ///
    /// Implementations should return `Ok(None)` if the backend has no nodes
    /// or if `size == 0`.
    fn get_index(
        &self,
        seed: Seed,
        size: usize,
        address: String,
        node_state: Option<&NodeStateStore>,
        chain_id: u64,
    ) -> Result<Option<(u64, Option<u64>)>>;

    /// Add a node to the backend. Backends should be idempotent on duplicates.
    fn add(&mut self, address: T);

    /// Remove a node from the backend. Removing a non-existent node is a no-op.
    fn remove(&mut self, address: T);

    /// Return all registered node addresses as hex strings.
    fn nodes(&self) -> Vec<String>;
}

/// Distance-sortition backend.
///
/// Stores a set of hex-encoded addresses and delegates committee selection
/// to `DistanceSortition`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistanceBackend {
    /// Registered node addresses (hex).
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
    /// Build the address list, run `DistanceSortition(seed, nodes, size)`,
    /// then check whether `address` is in the result.
    ///
    /// Returns `Ok(false)` if there are no nodes or `size == 0`.
    fn contains(
        &self,
        seed: Seed,
        size: usize,
        address: String,
        _node_state: Option<&NodeStateStore>,
        _chain_id: u64,
    ) -> anyhow::Result<bool> {
        if size == 0 {
            return Err(anyhow!("Size cannot be 0"));
        }

        if self.nodes.len() == 0 {
            return Err(anyhow!("No nodes registered!"));
        }

        let committee = get_committee(seed, size, self.nodes.clone())?;

        Ok(committee
            .iter()
            .any(|(_, addr)| addr.to_string() == address))
    }

    fn get_index(
        &self,
        seed: Seed,
        size: usize,
        address: String,
        _node_state: Option<&NodeStateStore>,
        _chain_id: u64,
    ) -> Result<Option<(u64, Option<u64>)>> {
        if size == 0 {
            return Err(anyhow!("Size cannot be 0"));
        }

        if self.nodes.len() == 0 {
            return Err(anyhow!("No nodes registered!"));
        }

        let committee = get_committee(seed, size, self.nodes.clone())?;

        let maybe = committee
            .iter()
            .enumerate()
            .find_map(|(i, (_, addr))| (addr.to_string() == address).then(|| (i as u64, None)));

        Ok(maybe)
    }

    /// Insert a node address (hex). Duplicate inserts are harmless.
    fn add(&mut self, address: String) {
        self.nodes.insert(address);
    }

    /// Remove a node address (hex). Missing entries are ignored.
    fn remove(&mut self, address: String) {
        self.nodes.remove(&address);
    }

    /// Return all node addresses as hex strings.
    fn nodes(&self) -> Vec<String> {
        self.nodes.iter().cloned().collect()
    }
}

fn get_committee(
    seed: Seed,
    size: usize,
    nodes: HashSet<String>,
) -> Result<Vec<(BigInt, Address)>> {
    let registered_nodes: Vec<Address> = nodes
        .into_iter()
        .map(|b| b.parse().context(format!("Error parsing address {}", b)))
        .collect::<Result<_>>()?;

    DistanceSortition::new(seed.into(), registered_nodes, size)
        .get_committee()
        .context("Could not get committee!")
}

/// Score-sortition backend.
///
/// Stores richer `RegisteredNode` entries (address + per-node ticket set).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoreBackend {
    /// Nodes with their ticket sets (used by score-based committee selection).
    registered: Vec<RegisteredNode>,
}

impl Default for ScoreBackend {
    fn default() -> Self {
        Self {
            registered: Vec::new(),
        }
    }
}

impl ScoreBackend {
    /// Build a vector of ephemeral nodes from the node state.
    ///
    /// The nodes are built from the node state and the registered nodes.
    fn build_nodes_from_state(
        &self,
        chain_id: u64,
        node_state: &NodeStateStore,
    ) -> Vec<RegisteredNode> {
        self.registered
            .iter()
            .filter_map(|n| {
                let addr_str = n.address.to_string();
                let key = (chain_id, addr_str.clone());
                let Some(ns) = node_state.nodes.get(&key) else {
                    return None;
                };
                if !ns.active {
                    return None;
                }

                let count = node_state.available_tickets(chain_id, &addr_str) as u64;
                if count == 0 {
                    return None;
                }

                let tickets = (1..=count).map(|i| Ticket { ticket_id: i }).collect();
                Some(RegisteredNode {
                    address: n.address,
                    tickets,
                })
            })
            .collect()
    }
}

impl SortitionList<String> for ScoreBackend {
    /// Compute score-based winners (`ScoreSortition`) and check if `address` is included.
    ///
    /// Returns `Ok(false)` if there are no nodes or `size == 0`.
    fn contains(
        &self,
        seed: Seed,
        size: usize,
        address: String,
        node_state: Option<&NodeStateStore>,
        chain_id: u64,
    ) -> anyhow::Result<bool> {
        if size == 0 {
            return Ok(false);
        }
        let Some(state) = node_state else {
            return Ok(false);
        };

        let nodes = self.build_nodes_from_state(chain_id, state);
        if nodes.is_empty() {
            return Ok(false);
        }

        let winners = ScoreSortition::new(size).get_committee(seed.into(), &nodes)?;
        let want: Address = address.parse()?;
        Ok(winners.iter().any(|w| w.address == want))
    }

    /// Compute score-based winners (`ScoreSortition`) and check if `address` is included.
    ///
    /// Returns `Ok(false)` if there are no nodes or `size == 0`.
    fn get_index(
        &self,
        seed: Seed,
        size: usize,
        address: String,
        node_state: Option<&NodeStateStore>,
        chain_id: u64,
    ) -> anyhow::Result<Option<(u64, Option<u64>)>> {
        if size == 0 {
            return Ok(None);
        }

        if node_state.is_none() {
            return Ok(None);
        }

        let nodes: Vec<RegisteredNode> = self.build_nodes_from_state(chain_id, node_state.unwrap());

        if nodes.is_empty() {
            return Ok(None);
        }

        let winners = ScoreSortition::new(size).get_committee(seed.into(), &nodes)?;
        let want: alloy::primitives::Address = address.parse()?;

        let maybe = winners
            .iter()
            .enumerate()
            .find_map(|(i, w)| (w.address == want).then(|| (i as u64, Some(w.ticket_id))));
        Ok(maybe)
    }

    /// Add a node, creating an empty ticket set when first seen.
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

    /// Remove the node (if present).
    ///
    /// Note: `used_ticket_ids` is a legacy field and clearing it here has
    /// no effect on current per-node ticket ID semantics.
    fn remove(&mut self, address: String) {
        if let Ok(addr) = address.parse::<Address>() {
            if let Some(i) = self.registered.iter().position(|n| n.address == addr) {
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

/// Enum wrapper around the supported backends.
///
/// New chains should default to `Distance`. If a chain is intended to
/// use score selection, construct it as `SortitionBackend::Score(ScoreBackend::default())`
/// and then populate tickets explicitly.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SortitionBackend {
    /// Distance-based selection (stores a simple set of addresses).
    Distance(DistanceBackend),
    /// Score-based selection (stores `RegisteredNode`s with tickets).
    Score(ScoreBackend),
}

impl SortitionBackend {
    /// Construct a backend preconfigured with a default `DistanceBackend`.
    pub fn default() -> Self {
        SortitionBackend::Distance(DistanceBackend::default())
    }
}

impl SortitionList<String> for SortitionBackend {
    fn contains(
        &self,
        seed: Seed,
        size: usize,
        address: String,
        node_state: Option<&NodeStateStore>,
        chain_id: u64,
    ) -> anyhow::Result<bool> {
        match self {
            SortitionBackend::Distance(b) => b.contains(seed, size, address, None, chain_id),
            SortitionBackend::Score(b) => b.contains(seed, size, address, node_state, chain_id),
        }
    }

    fn get_index(
        &self,
        seed: Seed,
        size: usize,
        address: String,
        node_state: Option<&NodeStateStore>,
        chain_id: u64,
    ) -> anyhow::Result<Option<(u64, Option<u64>)>> {
        match self {
            SortitionBackend::Distance(b) => b.get_index(seed, size, address, None, chain_id),
            SortitionBackend::Score(b) => b.get_index(seed, size, address, node_state, chain_id),
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

/// `Sortition` is an Actix actor that owns per-chain backends and exposes
/// message handlers to:
/// - add/remove nodes from a chain,
/// - list nodes for a chain,
/// - check committee membership for a chain.
///
/// Backends are persisted using `Persistable<HashMap<u64, SortitionBackend>>`
/// keyed by `chain_id`.
pub struct Sortition {
    /// Persistent map of `chain_id -> SortitionBackend`.
    list: Persistable<HashMap<u64, SortitionBackend>>,
    /// Event bus for error reporting and enclave event subscription.
    bus: Addr<EventBus<EnclaveEvent>>,
    /// Optional reference to node state for score-based sortition
    node_state: Option<Persistable<NodeStateStore>>,
}

/// Parameters for constructing a `Sortition` actor.
#[derive(Debug)]
pub struct SortitionParams {
    /// Event bus address.
    pub bus: Addr<EventBus<EnclaveEvent>>,
    /// Persisted per-chain backend map.
    pub list: Persistable<HashMap<u64, SortitionBackend>>,
    /// Optional node state for score-based sortition
    pub node_state: Option<Persistable<NodeStateStore>>,
}

impl Sortition {
    /// Construct a new `Sortition` actor with the given bus and repository.
    pub fn new(params: SortitionParams) -> Self {
        Self {
            list: params.list,
            bus: params.bus,
            node_state: params.node_state,
        }
    }

    /// Load persisted state, start the actor, and subscribe to `CiphernodeAdded/Removed`.
    ///
    /// The store is initialized with an empty `HashMap` if nothing is present.
    #[instrument(name = "sortition_attach", skip_all)]
    pub async fn attach(
        bus: &Addr<EventBus<EnclaveEvent>>,
        store: Repository<HashMap<u64, SortitionBackend>>,
    ) -> Result<Addr<Sortition>> {
        let list = store.load_or_default(HashMap::new()).await?;
        let addr = Sortition::new(SortitionParams {
            bus: bus.clone(),
            list,
            node_state: None, // Legacy attach without node state
        })
        .start();
        bus.do_send(Subscribe::new("CiphernodeAdded", addr.clone().into()));
        bus.do_send(Subscribe::new("CiphernodeRemoved", addr.clone().into()));
        Ok(addr)
    }

    /// Load persisted state with node state support for score-based sortition.
    ///
    /// This version allows score-based backends to query ticket availability.
    #[instrument(name = "sortition_attach_with_node_state", skip_all)]
    pub async fn attach_with_node_state(
        bus: &Addr<EventBus<EnclaveEvent>>,
        store: Repository<HashMap<u64, SortitionBackend>>,
        node_state_store: Repository<NodeStateStore>,
    ) -> Result<Addr<Sortition>> {
        let list = store.load_or_default(HashMap::new()).await?;
        let node_state = node_state_store
            .load_or_default(NodeStateStore::default())
            .await?;
        let addr = Sortition::new(SortitionParams {
            bus: bus.clone(),
            list,
            node_state: Some(node_state),
        })
        .start();
        bus.do_send(Subscribe::new("CiphernodeAdded", addr.clone().into()));
        bus.do_send(Subscribe::new("CiphernodeRemoved", addr.clone().into()));
        Ok(addr)
    }

    /// Return the current node addresses (hex) for `chain_id`.
    ///
    /// # Errors
    /// - Returns an error if the persisted map cannot be loaded from memory.
    /// - Returns an error if the given `chain_id` has no backend entry.
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
    type Context = actix::Context<Self>;
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

    /// Add a node to the target chain.
    ///
    /// If the chain does not exist yet, its backend is initialized to `Distance`.
    /// For score-based chains, switch construction time to `SortitionBackend::Score`
    /// and call the ticket setters separately (this handler only adds the address).
    #[instrument(name = "sortition_add_node", skip_all)]
    fn handle(&mut self, msg: CiphernodeAdded, _ctx: &mut Self::Context) -> Self::Result {
        trace!("Adding node: {}", msg.address);
        let chain_id = msg.chain_id;
        let addr = msg.address.clone();

        if let Err(err) = self.list.try_mutate(move |mut list_map| {
            list_map
                .entry(chain_id)
                .or_insert_with(SortitionBackend::default)
                .add(addr);
            Ok(list_map)
        }) {
            self.bus.err(EnclaveErrorType::Sortition, err);
        }
    }
}

impl Handler<CiphernodeRemoved> for Sortition {
    type Result = ();

    /// Remove a node from the target chain.
    ///
    /// If the chain entry is missing, nothing is created or removed.
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

impl Handler<GetNodeIndex> for Sortition {
    type Result = Option<(u64, Option<u64>)>;

    fn handle(&mut self, msg: GetNodeIndex, _ctx: &mut Self::Context) -> Self::Result {
        let node_state_snapshot = self.node_state.as_ref().and_then(|p| p.get());
        let node_state_ref = node_state_snapshot.as_ref();

        self.list
            .try_with(|map| {
                if let Some(backend) = map.get(&msg.chain_id) {
                    backend.get_index(
                        msg.seed,
                        msg.size,
                        msg.address.clone(),
                        node_state_ref,
                        msg.chain_id,
                    )
                } else {
                    Ok(None)
                }
            })
            .unwrap_or_else(|err| {
                self.bus.err(EnclaveErrorType::Sortition, err);
                None
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
