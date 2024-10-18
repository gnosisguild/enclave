use crate::DistanceSortition;
use actix::prelude::*;
use alloy::primitives::Address;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use data::{Checkpoint, FromSnapshotWithParams, Repository, Snapshot};
use enclave_core::{
    BusError, CiphernodeAdded, CiphernodeRemoved, EnclaveErrorType, EnclaveEvent, EventBus, Seed,
    Subscribe,
};
use std::collections::HashSet;

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "bool")]
pub struct GetHasNode {
    pub seed: Seed,
    pub address: String,
    pub size: usize,
}

pub trait SortitionList<T> {
    fn contains(&self, seed: Seed, size: usize, address: T) -> Result<bool>;
    fn add(&mut self, address: T);
    fn remove(&mut self, address: T);
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SortitionModule {
    nodes: HashSet<String>,
}

impl SortitionModule {
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
        }
    }
}

impl Default for SortitionModule {
    fn default() -> Self {
        Self::new()
    }
}

impl SortitionList<String> for SortitionModule {
    fn contains(&self, seed: Seed, size: usize, address: String) -> Result<bool> {
        if self.nodes.len() == 0 {
            return Err(anyhow!("No nodes registered!"));
        }

        let registered_nodes: Vec<Address> = self
            .nodes
            .clone()
            .into_iter()
            // TODO: error handling
            .map(|b| b.parse().unwrap())
            .collect();

        let Ok(committee) =
            DistanceSortition::new(seed.into(), registered_nodes, size).get_committee()
        else {
            return Err(anyhow!("Could not get committee!"));
        };

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
}

#[derive(Message)]
#[rtype(result = "Vec<String>")]
pub struct GetNodes;

pub struct Sortition {
    list: SortitionModule,
    bus: Addr<EventBus>,
    store: Repository<SortitionModule>,
}

pub struct SortitionParams {
    pub bus: Addr<EventBus>,
    pub store: Repository<SortitionModule>,
}

impl Sortition {
    pub fn new(params: SortitionParams) -> Self {
        Self {
            list: SortitionModule::new(),
            bus: params.bus,
            store: params.store,
        }
    }

    pub fn attach(bus: Addr<EventBus>, store: Repository<SortitionModule>) -> Addr<Sortition> {
        let addr = Sortition::new(SortitionParams {
            bus: bus.clone(),
            store,
        })
        .start();
        bus.do_send(Subscribe::new("CiphernodeAdded", addr.clone().into()));
        addr
    }

    pub fn get_nodes(&self) -> Vec<String> {
        self.list.nodes.clone().into_iter().collect()
    }
}

impl Actor for Sortition {
    type Context = actix::Context<Self>;
}

impl Snapshot for Sortition {
    type Snapshot = SortitionModule;
    fn snapshot(&self) -> Self::Snapshot {
        self.list.clone()
    }
}

#[async_trait]
impl FromSnapshotWithParams for Sortition {
    type Params = SortitionParams;
    async fn from_snapshot(params: Self::Params, snapshot: Self::Snapshot) -> Result<Self> {
        Ok(Sortition {
            bus: params.bus,
            store: params.store,
            list: snapshot,
        })
    }
}

impl Checkpoint for Sortition {
    fn repository(&self) -> Repository<SortitionModule> {
        self.store.clone()
    }
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
    fn handle(&mut self, msg: CiphernodeAdded, _ctx: &mut Self::Context) -> Self::Result {
        self.list.add(msg.address);
    }
}

impl Handler<CiphernodeRemoved> for Sortition {
    type Result = ();
    fn handle(&mut self, msg: CiphernodeRemoved, _ctx: &mut Self::Context) -> Self::Result {
        self.list.remove(msg.address);
    }
}

impl Handler<GetHasNode> for Sortition {
    type Result = bool;
    fn handle(&mut self, msg: GetHasNode, _ctx: &mut Self::Context) -> Self::Result {
        match self.list.contains(msg.seed, msg.size, msg.address) {
            Ok(val) => val,
            Err(err) => {
                self.bus.err(EnclaveErrorType::Sortition, err);
                false
            }
        }
    }
}

impl Handler<GetNodes> for Sortition {
    type Result = Vec<String>;

    fn handle(&mut self, _msg: GetNodes, _ctx: &mut Self::Context) -> Self::Result {
        self.get_nodes()
    }
}
