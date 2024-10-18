use async_trait::async_trait;
use data::{DataStore, Repository};
use enclave_core::E3id;

use crate::FheSnapshot;

#[derive(Clone)]
pub struct FheRepository {
    store: DataStore,
}

#[async_trait]
impl Repository for FheRepository {
    type State = FheSnapshot;
    fn store(&self) -> DataStore {
        self.store.clone()
    }
}

pub trait FheRepositoryFactory {
    fn fhe(&self, e3_id: &E3id) -> FheRepository;
}

impl FheRepositoryFactory for DataStore {
    fn fhe(&self, e3_id: &E3id) -> FheRepository {
        FheRepository {
            store: self.scope(format!("//fhe/{e3_id}")),
        }
    }
}
