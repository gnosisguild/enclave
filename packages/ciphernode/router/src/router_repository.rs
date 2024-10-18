use async_trait::async_trait;
use data::{DataStore, Repository};

use crate::E3RequestRouterSnapshot;

#[derive(Clone)]
pub struct E3RouterRepository {
    store: DataStore,
}

#[async_trait]
impl Repository for E3RouterRepository {
    type State = E3RequestRouterSnapshot;
    fn store(&self) -> DataStore {
        self.store.clone()
    }
}

pub trait E3RouterRepositoryFactory {
    fn router(&self) -> E3RouterRepository;
}

impl E3RouterRepositoryFactory for DataStore {
    fn router(&self) -> E3RouterRepository {
        E3RouterRepository {
            store: self.scope(format!("//router")),
        }
    }
}
