use config::StoreKeys;
use data::{Repositories, Repository};
use events::E3id;

use crate::{E3ContextSnapshot, E3Meta, E3RouterSnapshot};

pub trait MetaRepositoryFactory {
    fn meta(&self, e3_id: &E3id) -> Repository<E3Meta>;
}

impl MetaRepositoryFactory for Repositories {
    fn meta(&self, e3_id: &E3id) -> Repository<E3Meta> {
        Repository::new(self.store.scope(StoreKeys::meta(e3_id)))
    }
}

pub trait ContextRepositoryFactory {
    fn context(&self, e3_id: &E3id) -> Repository<E3ContextSnapshot>;
}

impl ContextRepositoryFactory for Repositories {
    fn context(&self, e3_id: &E3id) -> Repository<E3ContextSnapshot> {
        Repository::new(self.store.scope(StoreKeys::context(e3_id)))
    }
}

pub trait RouterRepositoryFactory {
    fn router(&self) -> Repository<E3RouterSnapshot>;
}

impl RouterRepositoryFactory for Repositories {
    fn router(&self) -> Repository<E3RouterSnapshot> {
        Repository::new(self.store.scope(StoreKeys::router()))
    }
}
