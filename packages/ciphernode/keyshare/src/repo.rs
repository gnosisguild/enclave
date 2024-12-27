use config::StoreKeys;
use data::{Repositories, Repository};
use enclave_core::E3id;

pub trait KeyshareRepositoryFactory {
    fn keyshare(&self, e3_id: &E3id) -> Repository<Vec<u8>>;
}

impl KeyshareRepositoryFactory for Repositories {
    fn keyshare(&self, e3_id: &E3id) -> Repository<Vec<u8>> {
        Repository::new(self.store.scope(StoreKeys::keyshare(e3_id)))
    }
}
