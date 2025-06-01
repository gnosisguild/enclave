use std::collections::HashMap;

use data::{Repositories, Repository};
use e3_config::StoreKeys;

use crate::SortitionModule;

pub trait SortitionRepositoryFactory {
    fn sortition(&self) -> Repository<HashMap<u64, SortitionModule>>;
}

impl SortitionRepositoryFactory for Repositories {
    fn sortition(&self) -> Repository<HashMap<u64, SortitionModule>> {
        Repository::new(self.store.scope(StoreKeys::sortition()))
    }
}
