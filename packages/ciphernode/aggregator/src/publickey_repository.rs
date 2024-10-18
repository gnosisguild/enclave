use crate::PublicKeyAggregatorState;
use async_trait::async_trait;
use data::{DataStore, Repository};
use enclave_core::E3id;

#[derive(Clone)]
pub struct PublicKeyAggregatorRepository {
    store: DataStore,
}

#[async_trait]
impl Repository for PublicKeyAggregatorRepository {
    type State = PublicKeyAggregatorState;
    fn store(&self) -> DataStore {
        self.store.clone()
    }
}

pub trait PublicKeyAggregatorRepositoryFactory {
    fn publickey(&self, e3_id: &E3id) -> PublicKeyAggregatorRepository;
}

impl PublicKeyAggregatorRepositoryFactory for DataStore {
    fn publickey(&self, e3_id: &E3id) -> PublicKeyAggregatorRepository {
        PublicKeyAggregatorRepository {
            store: self.scope(format!("//publickey/{e3_id}")),
        }
    }
}
