use async_trait::async_trait;
use data::{DataStore, Repository};
use enclave_core::E3id;

use crate::E3RequestContextSnapshot;

#[derive(Clone)]
pub struct E3ContextRepository {
    store: DataStore,
}

#[async_trait]
impl Repository for E3ContextRepository {
    type State = E3RequestContextSnapshot;
    fn store(&self) -> DataStore {
        self.store.clone()
    }
}

pub trait E3ContextRepositoryFactory {
    fn context(&self, e3_id: &E3id) -> E3ContextRepository;
}

impl E3ContextRepositoryFactory for DataStore {
    fn context(&self, e3_id: &E3id) -> E3ContextRepository {
        E3ContextRepository {
            store: self.scope(format!("//context/{e3_id}")),
        }
    }
}
