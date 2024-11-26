use config::StoreKeys;
use data::{Repositories, Repository};
use enclave_core::E3id;

use crate::{CommitteeMeta, E3RequestContextSnapshot, E3RequestRouterSnapshot};

pub trait MetaRepositoryFactory {
    fn meta(&self, e3_id: &E3id) -> Repository<CommitteeMeta>;
}

impl MetaRepositoryFactory for Repositories {
    fn meta(&self, e3_id: &E3id) -> Repository<CommitteeMeta> {
        Repository::new(self.store.scope(StoreKeys::meta(e3_id)))
    }
}

pub trait ContextRepositoryFactory {
    fn context(&self, e3_id: &E3id) -> Repository<E3RequestContextSnapshot>;
}

impl ContextRepositoryFactory for Repositories {
    fn context(&self, e3_id: &E3id) -> Repository<E3RequestContextSnapshot> {
        Repository::new(self.store.scope(StoreKeys::context(e3_id)))
    }
}

pub trait RouterRepositoryFactory {
    fn router(&self) -> Repository<E3RequestRouterSnapshot>;
}

impl RouterRepositoryFactory for Repositories {
    fn router(&self) -> Repository<E3RequestRouterSnapshot> {
        Repository::new(self.store.scope(StoreKeys::router()))
    }
}
