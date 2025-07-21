// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

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
