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

/// Which sortition algorithm to use.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SortitionType {
    Distance,
    Score,
}

/// Common operations required by sortition backends.
pub trait SortitionList<T> {
    fn contains(&self, seed: Seed, size: usize, address: T) -> Result<bool>;
    fn add(&mut self, address: T);
    fn remove(&mut self, address: T);
    fn nodes(&self) -> Vec<String>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistanceBackend {
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

    fn add(&mut self, address: String) {
        self.nodes.insert(address);
    }

    fn remove(&mut self, address: String) {
        self.nodes.remove(&address);
    }

    fn nodes(&self) -> Vec<String> {
        self.nodes.iter().cloned().collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoreBackend {
    registered: Vec<RegisteredNode>,
}

impl Default for ScoreBackend {
    fn default() -> Self {
        Self {
            registered: Vec::new(),
        }
    }
}

impl SortitionList<String> for ScoreBackend {
    fn contains(&self, seed: Seed, size: usize, address: String) -> Result<bool> {
        if self.registered.is_empty() || size == 0 {
            return Ok(false);
        }

        let winners = ScoreSortition::new(size).get_committee(seed.into(), &self.registered)?;
        let want: Address = address.parse()?;
        Ok(winners.iter().any(|w| w.address == want))
    }

    fn add(&mut self, address: String) {
        if let Ok(addr) = address.parse::<Address>() {
            if !self.registered.iter().any(|n| n.address == addr) {
                self.registered.push(RegisteredNode {
                    address: addr,
                    tickets: Vec::new(),
                });
            }
        }
    }

    fn remove(&mut self, address: String) {
        if let Ok(addr) = address.parse::<Address>() {
            if let Some(i) = self.registered.iter().position(|n| n.address == addr) {
                self.registered.swap_remove(i);
            }
        }
    }

    fn nodes(&self) -> Vec<String> {
        self.registered
            .iter()
            .map(|n| n.address.to_string())
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SortitionBackend {
    Distance(DistanceBackend),
    Score(ScoreBackend),
}

impl SortitionBackend {
    pub fn default_for(kind: SortitionType) -> Self {
        match kind {
            SortitionType::Distance => SortitionBackend::Distance(DistanceBackend::default()),
            SortitionType::Score => SortitionBackend::Score(ScoreBackend::default()),
        }
    }

    pub fn kind(&self) -> SortitionType {
        match self {
            SortitionBackend::Distance(_) => SortitionType::Distance,
            SortitionBackend::Score(_) => SortitionType::Score,
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

/// Actor holding per-chain sortition backends.
/// New chains use `default_sort` to choose which backend to initialize.
pub struct Sortition {
    list: Persistable<HashMap<u64, SortitionBackend>>,
    bus: Addr<EventBus<EnclaveEvent>>,
    sort_type: SortitionType,
}

#[derive(Debug)]
pub struct SortitionParams {
    pub bus: Addr<EventBus<EnclaveEvent>>,
    pub list: Persistable<HashMap<u64, SortitionBackend>>,
    pub sort_type: SortitionType,
}

impl Sortition {
    pub fn new(params: SortitionParams) -> Self {
        Self {
            list: params.list,
            bus: params.bus,
            sort_type: params.sort_type,
        }
    }

    /// Start the actor and subscribe to add/remove events.
    #[instrument(name = "sortition", skip_all)]
    pub async fn attach(
        bus: &Addr<EventBus<EnclaveEvent>>,
        store: Repository<HashMap<u64, SortitionBackend>>,
        sort_type: SortitionType,
    ) -> Result<Addr<Sortition>> {
        let list = store.load_or_default(HashMap::new()).await?;
        let addr = Sortition::new(SortitionParams {
            bus: bus.clone(),
            list,
            sort_type,
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
        let backend = list_by_chain_id
            .get(&chain_id)
            .ok_or(anyhow!("No list found for chain_id {}", chain_id))?;
        Ok(backend.nodes())
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

    #[instrument(name = "sortition", skip_all)]
    fn handle(&mut self, msg: CiphernodeAdded, _ctx: &mut Self::Context) -> Self::Result {
        trace!("Adding node: {}", msg.address);
        let sort_type = self.sort_type;
        let chain_id = msg.chain_id;
        let addr = msg.address.clone();

        if let Err(err) = self.list.try_mutate(move |mut list_map| {
            list_map
                .entry(chain_id)
                .or_insert_with(|| SortitionBackend::default_for(sort_type))
                .add(addr);
            Ok(list_map)
        }) {
            self.bus.err(EnclaveErrorType::Sortition, err);
        }
    }
}

impl Handler<CiphernodeRemoved> for Sortition {
    type Result = ();

    #[instrument(name = "sortition", skip_all)]
    fn handle(&mut self, msg: CiphernodeRemoved, _ctx: &mut Self::Context) -> Self::Result {
        info!("Removing node: {}", msg.address);
        let sort_type = self.sort_type;
        let chain_id = msg.chain_id;
        let addr = msg.address.clone();

        if let Err(err) = self.list.try_mutate(move |mut list_map| {
            list_map
                .entry(chain_id)
                .or_insert_with(|| SortitionBackend::default_for(sort_type))
                .remove(addr);
            Ok(list_map)
        }) {
            self.bus.err(EnclaveErrorType::Sortition, err);
        }
    }
}

impl Handler<GetHasNode> for Sortition {
    type Result = bool;

    #[instrument(name = "sortition", skip_all)]
    fn handle(&mut self, msg: GetHasNode, _ctx: &mut Self::Context) -> Self::Result {
        self.list
            .try_with(|list_map| {
                if let Some(backend) = list_map.get(&msg.chain_id) {
                    return backend.contains(msg.seed, msg.size, msg.address);
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
