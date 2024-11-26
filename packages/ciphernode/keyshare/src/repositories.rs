use config::StoreKeys;
use data::{Repositories, Repository};
use enclave_core::E3id;

use crate::KeyshareState;

pub trait KeyshareRepositoryFactory {
    fn keyshare(&self, e3_id: &E3id) -> Repository<KeyshareState>;
}

impl KeyshareRepositoryFactory for Repositories {
    fn keyshare(&self, e3_id: &E3id) -> Repository<KeyshareState> {
        Repository::new(self.store.scope(StoreKeys::keyshare(e3_id)))
    }
}
