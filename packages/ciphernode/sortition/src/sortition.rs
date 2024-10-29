use crate::DistanceSortition;
use actix::prelude::*;
use alloy::primitives::Address;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use data::{Checkpoint, FromSnapshotWithParams, Repository, Snapshot};
use enclave_core::{
    BusError, Die, EnclaveErrorType, EnclaveEvent, EventBus, EventId, Seed, Subscribe, Unsubscribe,
};
use std::collections::HashSet;
use tracing::trace;

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

impl SortitionList<&str> for SortitionModule {
    fn contains(&self, seed: Seed, size: usize, address: &str) -> Result<bool> {
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

    fn add(&mut self, address: &str) {
        self.nodes.insert(address.to_string());
    }

    fn remove(&mut self, address: &str) {
        self.nodes.remove(address);
    }
}

#[derive(Message)]
#[rtype(result = "Vec<String>")]
pub struct GetNodes;

pub struct Sortition {
    list: SortitionModule,
    bus: Addr<EventBus>,
    store: Repository<SortitionSnapshot>,
    processed: HashSet<EventId>,
}

pub struct SortitionParams {
    pub bus: Addr<EventBus>,
    pub store: Repository<SortitionSnapshot>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SortitionSnapshot {
    pub list: SortitionModule,
    pub processed: HashSet<EventId>,
}

impl Sortition {
    pub fn new(params: SortitionParams) -> Self {
        Self {
            list: SortitionModule::new(),
            bus: params.bus,
            store: params.store,
            processed: HashSet::new(),
        }
    }

    pub async fn load(
        bus: &Addr<EventBus>,
        store: &Repository<SortitionSnapshot>,
    ) -> Result<Addr<Sortition>> {
        let addr = if let Some(snapshot) = store.read().await? {
            Sortition::from_snapshot(
                SortitionParams {
                    bus: bus.clone(),
                    store: store.clone(),
                },
                snapshot,
            )
            .await?
            .start()
        } else {
            Sortition::new(SortitionParams {
                bus: bus.clone(),
                store: store.clone(),
            })
            .start()
        };

        bus.do_send(Subscribe::new("CiphernodeAdded", addr.clone().into()));
        bus.do_send(Subscribe::new("CiphernodeRemoved", addr.clone().into()));
        Ok(addr)
    }

    pub fn get_nodes(&self) -> Vec<String> {
        self.list.nodes.clone().into_iter().collect()
    }
}

impl Actor for Sortition {
    type Context = actix::Context<Self>;
}

impl Snapshot for Sortition {
    type Snapshot = SortitionSnapshot;
    fn snapshot(&self) -> Self::Snapshot {
        SortitionSnapshot {
            list: self.list.clone(),
            processed: self.processed.clone(),
        }
    }
}

#[async_trait]
impl FromSnapshotWithParams for Sortition {
    type Params = SortitionParams;
    async fn from_snapshot(params: Self::Params, snapshot: Self::Snapshot) -> Result<Self> {
        Ok(Sortition {
            bus: params.bus,
            store: params.store,
            list: snapshot.list,
            processed: snapshot.processed,
        })
    }
}

impl Checkpoint for Sortition {
    fn repository(&self) -> Repository<SortitionSnapshot> {
        self.store.clone()
    }
}

impl Handler<EnclaveEvent> for Sortition {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        if self.processed.contains(&msg.get_id()) {
            trace!(
                "Skipping processing event {} as has been seen before.",
                msg.get_id()
            );
            return;
        };

        match &msg {
            EnclaveEvent::CiphernodeAdded { data, .. } => self.list.add(&data.address),
            EnclaveEvent::CiphernodeRemoved { data, .. } => self.list.remove(&data.address),
            _ => (),
        }

        // Store processed event
        self.processed.insert(msg.get_id());
        self.checkpoint();
    }
}

impl Handler<GetHasNode> for Sortition {
    type Result = bool;
    fn handle(&mut self, msg: GetHasNode, _ctx: &mut Self::Context) -> Self::Result {
        match self.list.contains(msg.seed, msg.size, &msg.address) {
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

impl Handler<Die> for Sortition {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        self.bus
            .do_send(Unsubscribe::new("CiphernodeAdded", ctx.address().into()));

        self.bus
            .do_send(Unsubscribe::new("CiphernodeRemoved", ctx.address().into()));

        ctx.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::{Sortition, SortitionSnapshot};
    use actix::{clock::sleep, Actor};
    use alloy::primitives::Address;
    use anyhow::{bail, Result};
    use data::{DataStore, Repository};
    use enclave_core::{CiphernodeAdded, CiphernodeRemoved, Die, EnclaveEvent, EventBus};
    use rand::Rng;
    use std::time::Duration;

    fn generate_random_address() -> String {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 20];
        rng.fill(&mut bytes);
        let address = Address::from_slice(&bytes);
        format!("{:?}", address)
    }

    #[actix::test]
    async fn test_sortition_hydration() -> Result<()> {
        let store = DataStore::in_mem();
        let repo: Repository<SortitionSnapshot> =
            Repository::new(store.scope(format!("//sortition")));

        let bus_1 = EventBus::new(true).start();
        let sortition = Sortition::load(&bus_1, &repo).await?;

        let mut num_nodes = 0;
        let adds: Vec<CiphernodeAdded> = (0..6)
            .map(|i| {
                num_nodes += 1;
                CiphernodeAdded {
                    address: generate_random_address(),
                    index: i,
                    num_nodes,
                }
            })
            .collect();
        let removes: Vec<CiphernodeRemoved> = (0..6)
            .map(|i| {
                num_nodes -= 1;
                CiphernodeRemoved {
                    address: generate_random_address(),
                    index: i,
                    num_nodes,
                }
            })
            .collect();

        for event in adds.iter() {
            bus_1.do_send(EnclaveEvent::from(event.clone()));
        }

        sleep(Duration::from_millis(1)).await;

        sortition.do_send(Die);

        let Some(snapshot) = repo.read().await? else {
            bail!("Snapshot must exit")
        };

        assert_eq!(snapshot.processed.len(), 6);

        let bus_2 = EventBus::new(true).start();

        Sortition::load(&bus_2, &repo).await?;

        for event in adds.iter() {
            bus_2.do_send(EnclaveEvent::from(event.clone()));
        }

        sleep(Duration::from_millis(1)).await;

        let Some(snapshot) = repo.read().await? else {
            bail!("Snapshot must exit")
        };

        assert_eq!(
            snapshot.processed.len(),
            6,
            "Snapshot events should not have changed"
        );

        for event in removes.iter() {
            bus_2.do_send(EnclaveEvent::from(event.clone()));
        }

        sleep(Duration::from_millis(1)).await;

        let Some(snapshot) = repo.read().await? else {
            bail!("Snapshot must exit")
        };

        assert_eq!(snapshot.processed.len(), 12);

        Ok(())
    }
}
