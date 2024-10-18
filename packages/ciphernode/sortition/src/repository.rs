use crate::SortitionModule;
use anyhow::*;
use async_trait::async_trait;
use data::{DataStore, Repository};

#[derive(Clone)]
pub struct SortitionRepository {
    store: DataStore,
}

#[async_trait]
impl Repository for SortitionRepository {
    type State = SortitionModule;
    async fn read(&self) -> Result<Option<SortitionModule>> {
        self.store.read().await
    }

    fn write(&self, value: &SortitionModule) {
        self.store.write(value)
    }

    fn store(&self) -> DataStore {
        self.store.clone()
    }
}

pub trait SortitionRepositoryFactory {
    fn sortition(&self) -> SortitionRepository;
}

impl SortitionRepositoryFactory for DataStore {
    fn sortition(&self) -> SortitionRepository {
        SortitionRepository {
            store: self.scope(format!("//sortition")),
        }
    }
}
