// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::backends::SortitionBackend;
use crate::sortition::NodeStateStore;
use e3_config::StoreKeys;
use e3_data::{Repositories, Repository};
use e3_events::E3id;
use std::collections::HashMap;

pub trait SortitionRepositoryFactory {
    fn sortition(&self) -> Repository<HashMap<u64, SortitionBackend>>;
}

impl SortitionRepositoryFactory for Repositories {
    fn sortition(&self) -> Repository<HashMap<u64, SortitionBackend>> {
        Repository::new(self.store.scope(StoreKeys::sortition()))
    }
}

pub trait NodeStateRepositoryFactory {
    fn node_state(&self) -> Repository<HashMap<u64, NodeStateStore>>;
}

impl NodeStateRepositoryFactory for Repositories {
    fn node_state(&self) -> Repository<HashMap<u64, NodeStateStore>> {
        Repository::new(self.store.scope(StoreKeys::node_state()))
    }
}

pub trait FinalizedCommitteesRepositoryFactory {
    fn finalized_committees(&self) -> Repository<HashMap<E3id, Vec<String>>>;
}

impl FinalizedCommitteesRepositoryFactory for Repositories {
    fn finalized_committees(&self) -> Repository<HashMap<E3id, Vec<String>>> {
        Repository::new(self.store.scope(StoreKeys::finalized_committees()))
    }
}
