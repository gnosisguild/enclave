use async_trait::async_trait;
use data::{DataStore, Repository};
use enclave_core::E3id;

use crate::CommitteeMeta;

#[derive(Clone)]
pub struct CommitteeMetaRepository {
    store: DataStore,
}

#[async_trait]
impl Repository for CommitteeMetaRepository {
    type State = CommitteeMeta;
    fn store(&self) -> DataStore {
        self.store.clone()
    }
}

pub trait CommitteeMetaRepositoryFactory {
    fn meta(&self, e3_id: &E3id) -> CommitteeMetaRepository;
}

impl CommitteeMetaRepositoryFactory for DataStore {
    fn meta(&self, e3_id: &E3id) -> CommitteeMetaRepository {
        CommitteeMetaRepository {
            store: self.scope(format!("//meta/{e3_id}")),
        }
    }
}
