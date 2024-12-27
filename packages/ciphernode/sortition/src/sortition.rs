use crate::DistanceSortition;
use actix::prelude::*;
use alloy::primitives::Address;
use anyhow::{anyhow, Result};
use data::{AutoPersist, Persistable, Repository};
use enclave_core::{
    get_tag, BusError, CiphernodeAdded, CiphernodeRemoved, EnclaveErrorType, EnclaveEvent,
    EventBus, Seed, Subscribe,
};
use std::collections::HashSet;
use tracing::{info, instrument};

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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SortitionModule {
    nodes: HashSet<String>,
}

impl SortitionModule {
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
        }
    }

    pub fn nodes(&self) -> &HashSet<String> {
        &self.nodes
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
    list: Persistable<SortitionModule>,
    bus: Addr<EventBus>,
}

#[derive(Debug)]
pub struct SortitionParams {
    bus: Addr<EventBus>,
    list: Persistable<SortitionModule>,
}

impl Sortition {
    pub fn new(params: SortitionParams) -> Self {
        Self {
            list: params.list,
            bus: params.bus,
        }
    }

    #[instrument(name="sortition", skip_all, fields(id = get_tag()))]
    pub async fn attach(
        bus: &Addr<EventBus>,
        store: Repository<SortitionModule>,
    ) -> Result<Addr<Sortition>> {
        let list = store.load_or_default(SortitionModule::default()).await?;
        let addr = Sortition::new(SortitionParams {
            bus: bus.clone(),
            list,
        })
        .start();
        bus.do_send(Subscribe::new("CiphernodeAdded", addr.clone().into()));
        Ok(addr)
    }

    pub fn get_nodes(&self) -> Vec<String> {
        self.list.get().unwrap().nodes.clone().into_iter().collect()
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

    #[instrument(name="sortition", skip_all, fields(id = get_tag()))]
    fn handle(&mut self, msg: CiphernodeAdded, _ctx: &mut Self::Context) -> Self::Result {
        info!("Adding node: {}", msg.address);
        match self.list.try_mutate(|mut list| {
            list.add(msg.address);
            Ok(list)
        }) {
            Err(err) => self.bus.err(EnclaveErrorType::Sortition, err),
            _ => (),
        };
    }
}

impl Handler<CiphernodeRemoved> for Sortition {
    type Result = ();

    #[instrument(name="sortition", skip_all, fields(id = get_tag()))]
    fn handle(&mut self, msg: CiphernodeRemoved, _ctx: &mut Self::Context) -> Self::Result {
        info!("Removing node: {}", msg.address);
        match self.list.try_mutate(|mut list| {
            list.remove(msg.address);
            Ok(list)
        }) {
            Err(err) => self.bus.err(EnclaveErrorType::Sortition, err),
            _ => (),
        };
    }
}

impl Handler<GetHasNode> for Sortition {
    type Result = bool;

    #[instrument(name="sortition", skip_all, fields(id = get_tag()))]
    fn handle(&mut self, msg: GetHasNode, _ctx: &mut Self::Context) -> Self::Result {
        self.list
            .try_with(|list| list.contains(msg.seed, msg.size, msg.address))
            .unwrap_or_else(|err| {
                self.bus.err(EnclaveErrorType::Sortition, err);
                false
            })
    }
}

impl Handler<GetNodes> for Sortition {
    type Result = Vec<String>;

    fn handle(&mut self, _msg: GetNodes, _ctx: &mut Self::Context) -> Self::Result {
        self.get_nodes()
    }
}
