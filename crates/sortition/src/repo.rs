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
use e3_request::E3Meta;
use std::collections::HashMap;

pub trait SortitionRepositoryFactory {
    fn sortition(&self) -> Repository<HashMap<u64, SortitionBackend>>;
}

impl SortitionRepositoryFactory for Repositories {
    /// Create a Repository scoped to the sortition store.
    ///
    /// This repository provides access to the map keyed by `u64` with `SortitionBackend` values
    /// stored under the sortition namespace.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given a `repositories` value, obtain the sortition repository.
    /// let sortition_repo = repositories.sortition();
    /// ```
    fn sortition(&self) -> Repository<HashMap<u64, SortitionBackend>> {
        Repository::new(self.store.scope(StoreKeys::sortition()))
    }
}

pub trait CiphernodeSelectorFactory {
    fn ciphernode_selector(&self) -> Repository<HashMap<E3id, E3Meta>>;
}

impl CiphernodeSelectorFactory for Repositories {
    /// Create a repository scoped to the `ciphernode_selector` store key.
    ///
    /// The returned repository stores a `HashMap<E3id, E3Meta>`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use your_crate::Repositories;
    /// let repos: Repositories = unimplemented!();
    /// let repo = repos.ciphernode_selector();
    /// ```
    fn ciphernode_selector(&self) -> Repository<HashMap<E3id, E3Meta>> {
        Repository::new(self.store.scope(StoreKeys::ciphernode_selector()))
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