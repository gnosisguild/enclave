// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_data::{Repositories, Repository};
use e3_events::{E3id, StoreKeys};

use crate::{PublicKeyAggregatorState, ThresholdPlaintextAggregatorState};

pub trait TrBfvPlaintextRepositoryFactory {
    fn trbfv_plaintext(&self, e3_id: &E3id) -> Repository<ThresholdPlaintextAggregatorState>;
}

impl TrBfvPlaintextRepositoryFactory for Repositories {
    fn trbfv_plaintext(&self, e3_id: &E3id) -> Repository<ThresholdPlaintextAggregatorState> {
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
