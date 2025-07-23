// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_config::StoreKeys;
use e3_data::{Repositories, Repository};
use e3_events::E3id;

pub trait KeyshareRepositoryFactory {
    fn keyshare(&self, e3_id: &E3id) -> Repository<Vec<u8>>;
}

impl KeyshareRepositoryFactory for Repositories {
    fn keyshare(&self, e3_id: &E3id) -> Repository<Vec<u8>> {
        Repository::new(self.store.scope(StoreKeys::keyshare(e3_id)))
    }
}
