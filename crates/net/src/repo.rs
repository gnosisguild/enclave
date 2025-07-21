// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_config::StoreKeys;
use e3_data::{Repositories, Repository};

pub trait NetRepositoryFactory {
    fn libp2p_keypair(&self) -> Repository<Vec<u8>>;
}

impl NetRepositoryFactory for Repositories {
    fn libp2p_keypair(&self) -> Repository<Vec<u8>> {
        Repository::new(self.store.scope(StoreKeys::libp2p_keypair()))
    }
}
