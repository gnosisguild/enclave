use config::StoreKeys;
use data::{Repositories, Repository};

use crate::SortitionModule;

pub trait SortitionRepositoryFactory {
    fn sortition(&self) -> Repository<SortitionModule>;
}

impl SortitionRepositoryFactory for Repositories {
    fn sortition(&self) -> Repository<SortitionModule> {
        Repository::new(self.store.scope(StoreKeys::sortition()))
    }
}
