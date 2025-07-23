// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_config::StoreKeys;
use e3_data::{Repositories, Repository};
use e3_events::E3id;

use crate::{PlaintextAggregatorState, PublicKeyAggregatorState};

pub trait PlaintextRepositoryFactory {
    fn plaintext(&self, e3_id: &E3id) -> Repository<PlaintextAggregatorState>;
}

impl PlaintextRepositoryFactory for Repositories {
    fn plaintext(&self, e3_id: &E3id) -> Repository<PlaintextAggregatorState> {
        Repository::new(self.store.scope(StoreKeys::plaintext(e3_id)))
    }
}

pub trait PublicKeyRepositoryFactory {
    fn publickey(&self, e3_id: &E3id) -> Repository<PublicKeyAggregatorState>;
}

impl PublicKeyRepositoryFactory for Repositories {
    fn publickey(&self, e3_id: &E3id) -> Repository<PublicKeyAggregatorState> {
        Repository::new(self.store.scope(StoreKeys::publickey(e3_id)))
    }
}
