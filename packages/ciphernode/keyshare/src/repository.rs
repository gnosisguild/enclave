use crate::keyshare::KeyshareState;
use async_trait::async_trait;
use data::{DataStore, Repository};
use enclave_core::E3id;

#[derive(Clone)]
pub struct KeyshareRepository {
    store: DataStore,
}

#[async_trait]
impl Repository for KeyshareRepository {
    type State = KeyshareState;
    fn store(&self) -> DataStore {
        self.store.clone()
    }
}

pub trait KeyshareRepositoryFactory {
    fn keyshare(&self, e3_id: &E3id) -> KeyshareRepository;
}

impl KeyshareRepositoryFactory for DataStore {
    fn keyshare(&self, e3_id: &E3id) -> KeyshareRepository {
        KeyshareRepository {
            store: self.scope(format!("//keyshare/{e3_id}")),
        }
    }
}
