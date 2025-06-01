use std::collections::HashMap;

use e3_config::StoreKeys;
use e3_data::{Repositories, Repository};

use crate::SortitionModule;

pub trait SortitionRepositoryFactory {
    fn sortition(&self) -> Repository<HashMap<u64, SortitionModule>>;
}

impl SortitionRepositoryFactory for Repositories {
    fn sortition(&self) -> Repository<HashMap<u64, SortitionModule>> {
        Repository::new(self.store.scope(StoreKeys::sortition()))
    }
}
