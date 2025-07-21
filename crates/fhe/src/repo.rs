// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_config::StoreKeys;
use e3_data::{Repositories, Repository};
use e3_events::E3id;

use crate::FheSnapshot;

pub trait FheRepositoryFactory {
    fn fhe(&self, e3_id: &E3id) -> Repository<FheSnapshot>;
}

impl FheRepositoryFactory for Repositories {
    fn fhe(&self, e3_id: &E3id) -> Repository<FheSnapshot> {
        Repository::new(self.store.scope(StoreKeys::fhe(e3_id)))
    }
}
