use crate::PlaintextAggregatorState;
use async_trait::async_trait;
use data::{DataStore, Repository};
use enclave_core::E3id;

#[derive(Clone)]
pub struct PlaintextAggregatorRepository {
    store: DataStore,
}

#[async_trait]
impl Repository for PlaintextAggregatorRepository {
    type State = PlaintextAggregatorState;
    fn store(&self) -> DataStore {
        self.store.clone()
    }
}

pub trait PlaintextAggregatorRepositoryFactory {
    fn plaintext(&self, e3_id: &E3id) -> PlaintextAggregatorRepository;
}

impl PlaintextAggregatorRepositoryFactory for DataStore {
    fn plaintext(&self, e3_id: &E3id) -> PlaintextAggregatorRepository {
        PlaintextAggregatorRepository {
            store: self.scope(format!("//plaintext/{e3_id}")),
        }
    }
}
