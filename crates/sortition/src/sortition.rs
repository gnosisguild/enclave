// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::distance::DistanceSortition;
use crate::ticket::RegisteredNode;
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
use std::collections::{HashMap, HashSet};
use tracing::{info, instrument, trace};

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "bool")]
pub struct GetHasNode {
    pub seed: Seed,
    pub address: String,
    pub size: usize,
    pub chain_id: u64,
}

#[derive(Message)]
#[rtype(result = "Vec<String>")]
pub struct GetNodes {
    pub chain_id: u64,
}

/// Set the algorithm used by a chain (Distance or Score).
#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "bool")]
pub struct SetSortitionType {
    pub chain_id: u64,
    pub sort_type: SortitionType,
}

/// Which sortition algorithm a chain uses.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SortitionType {
    Distance,
    Score,
}

/// Unified list behavior for sortition backends.
pub trait SortitionList<T> {
    fn contains(&self, seed: Seed, size: usize, address: T) -> Result<bool>;
    fn add(&mut self, address: T);
    fn remove(&mut self, address: T);
    fn nodes(&self) -> Vec<String>;
}

/// Backend for a single chain: either Distance or Score variant.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SortitionBackend {
    Distance { nodes: HashSet<String> },
    Score { registered: Vec<RegisteredNode> },
}

impl Default for SortitionBackend {
    fn default() -> Self {
        SortitionBackend::Distance {
            nodes: HashSet::new(),
        }
    }
}

impl SortitionBackend {
    fn _sortition_type(&self) -> SortitionType {
        match self {
            SortitionBackend::Distance { .. } => SortitionType::Distance,
            SortitionBackend::Score { .. } => SortitionType::Score,
        }
    }

    fn _as_distance_mut(&mut self) -> &mut HashSet<String> {
        match self {
            SortitionBackend::Distance { nodes } => nodes,
            SortitionBackend::Score { .. } => {
                panic!("attempted to mutate Distance nodes on Score backend")
            }
        }
    }

    fn _as_score_mut(&mut self) -> &mut Vec<RegisteredNode> {
        match self {
            SortitionBackend::Score { registered } => registered,
            SortitionBackend::Distance { .. } => {
                panic!("attempted to mutate Score registered on Distance backend")
            }
        }
    }

    fn nodes_view(&self) -> Vec<String> {
        match self {
            SortitionBackend::Distance { nodes } => nodes.iter().cloned().collect(),
            SortitionBackend::Score { registered } => {
                registered.iter().map(|n| n.address.to_string()).collect()
            }
        }
    }

    fn contains_distance(&self, seed: Seed, size: usize, address: &str) -> Result<bool> {
        let nodes = match self {
            SortitionBackend::Distance { nodes } => nodes,
            _ => return Err(anyhow!("wrong backend for distance contains")),
        };

        if nodes.is_empty() || size == 0 {
            return Ok(false);
        }

        let registered_nodes: Vec<Address> = nodes
            .iter()
            .cloned()
            .map(|b| b.parse::<Address>())
            .collect::<Result<_, _>>()?;

        let committee =
            DistanceSortition::new(seed.into(), registered_nodes, size).get_committee()?;

        Ok(committee
            .iter()
            .any(|(_, addr)| addr.to_string() == address))
    }

    fn contains_score(&self, seed: Seed, size: usize, address: &str) -> Result<bool> {
        let registered = match self {
            SortitionBackend::Score { registered } => registered,
            _ => return Err(anyhow!("wrong backend for score contains")),
        };

        if registered.is_empty() || size == 0 {
            return Ok(false);
        }

        let committee = ScoreSortition::new(size).get_committee(seed.into(), registered)?;
        let want: Address = address.parse()?;
        Ok(committee.iter().any(|w| w.address == want))
    }
}

/// Per-chain module: stores the chosen algorithm and its backend storage.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SortitionModule {
    sortition: SortitionType,
    backend: SortitionBackend,
}

impl Default for SortitionModule {
    fn default() -> Self {
        Self {
            sortition: SortitionType::Distance,
            backend: SortitionBackend::default(),
        }
    }
}

impl SortitionModule {
    /// Create a module with a specific algorithm.
    pub fn new(sortition: SortitionType) -> Self {
        let backend = match sortition {
            SortitionType::Distance => SortitionBackend::Distance {
                nodes: HashSet::new(),
            },
            SortitionType::Score => SortitionBackend::Score {
                registered: Vec::new(),
            },
        };
        Self { sortition, backend }
    }

    pub fn sort_type(&self) -> SortitionType {
        self.sortition
    }

    /// Switch algorithm and migrate storage shape.
    pub fn set_sort_type(&mut self, sort_type: SortitionType) {
        if self.sortition == sort_type {
            return;
        }
        self.backend = match sort_type {
            SortitionType::Distance => SortitionBackend::Distance {
                nodes: self.backend.nodes_view().into_iter().collect(),
            },
            SortitionType::Score => SortitionBackend::Score {
                registered: self
                    .backend
                    .nodes_view()
                    .into_iter()
                    .filter_map(|s| s.parse::<Address>().ok())
                    .map(|address| RegisteredNode {
                        address,
                        tickets: Vec::new(),
                    })
                    .collect(),
            },
        };
        self.sortition = sort_type;
    }
}

impl SortitionList<String> for SortitionModule {
    fn contains(&self, seed: Seed, size: usize, address: String) -> Result<bool> {
        match self.sortition {
            SortitionType::Distance => self.backend.contains_distance(seed, size, &address),
            SortitionType::Score => self.backend.contains_score(seed, size, &address),
        }
    }

    fn add(&mut self, address: String) {
        match (&mut self.backend, self.sortition) {
            (SortitionBackend::Distance { nodes }, SortitionType::Distance) => {
                nodes.insert(address);
            }
            (SortitionBackend::Score { registered }, SortitionType::Score) => {
                if let Ok(addr) = address.parse::<Address>() {
                    if !registered.iter().any(|n| n.address == addr) {
                        registered.push(RegisteredNode {
                            address: addr,
                            tickets: Vec::new(),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    fn remove(&mut self, address: String) {
        match (&mut self.backend, self.sortition) {
            (SortitionBackend::Distance { nodes }, SortitionType::Distance) => {
                nodes.remove(&address);
            }
            (SortitionBackend::Score { registered }, SortitionType::Score) => {
                if let Ok(addr) = address.parse::<Address>() {
                    if let Some(i) = registered.iter().position(|n| n.address == addr) {
                        registered.swap_remove(i);
                    }
                }
            }
            _ => {}
        }
    }

    fn nodes(&self) -> Vec<String> {
        self.backend.nodes_view()
    }
}

/// Actor holding per-chain sortition modules.
pub struct Sortition {
    list: Persistable<HashMap<u64, SortitionModule>>,
    bus: Addr<EventBus<EnclaveEvent>>,
}

#[derive(Debug)]
pub struct SortitionParams {
    pub bus: Addr<EventBus<EnclaveEvent>>,
    pub list: Persistable<HashMap<u64, SortitionModule>>,
}

impl Sortition {
    pub fn new(params: SortitionParams) -> Self {
        Self {
            list: params.list,
            bus: params.bus,
        }
    }

    /// Start the actor and subscribe to events.
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

    /// Return node addresses for a chain.
    pub fn get_nodes(&self, chain_id: u64) -> Result<Vec<String>> {
        let list_by_chain_id = self.list.get().ok_or(anyhow!(
            "Could not get sortition's list cache. This should not happen."
        ))?;
        let list = list_by_chain_id
            .get(&chain_id)
            .ok_or(anyhow!("No list found for chain_id {}", chain_id))?;
        Ok(list.nodes())
    }

    /// Set the algorithm for a chain.
    pub fn set_sort_type(&mut self, chain_id: u64, sort_type: SortitionType) -> Result<()> {
        self.list.try_mutate(|mut map| {
            map.entry(chain_id)
                .and_modify(|m| m.set_sort_type(sort_type))
                .or_insert_with(|| SortitionModule::new(sort_type));
            Ok(map)
        })
    }
}

impl Actor for Sortition {
    type Context = Context<Self>;
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

    /// Add a node to the chain’s backend.
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

    /// Remove a node from the chain’s backend.
    #[instrument(name = "sortition", skip_all)]
    fn handle(&mut self, msg: CiphernodeRemoved, _ctx: &mut Self::Context) -> Self::Result {
        info!("Removing node: {}", msg.address);
        match self.list.try_mutate(|mut list_map| {
            list_map
                .get_mut(&msg.chain_id)
                .ok_or(anyhow!(
                    "Cannot remove a node from list that does not exist. It appears that the list for chain_id '{}' has not yet been created.",
                    &msg.chain_id
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

    /// Check committee membership for a node.
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

    fn handle(&mut self, msg: GetNodes, _ctx: &mut Self::Context) -> Self::Result {
        self.get_nodes(msg.chain_id).unwrap_or_default()
    }
}

impl Handler<SetSortitionType> for Sortition {
    type Result = bool;

    fn handle(&mut self, msg: SetSortitionType, _ctx: &mut Self::Context) -> Self::Result {
        self.set_sort_type(msg.chain_id, msg.sort_type)
            .map(|_| true)
            .unwrap_or_else(|err| {
                self.bus.err(EnclaveErrorType::Sortition, err);
                false
            })
    }
}
