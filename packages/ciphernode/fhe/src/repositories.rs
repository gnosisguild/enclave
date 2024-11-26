
use config::StoreKeys;
use data::{Repositories, Repository};
use enclave_core::E3id;

use crate::FheSnapshot;

pub trait FheRepositoryFactory {
    fn fhe(&self, e3_id: &E3id) -> Repository<FheSnapshot>;
}

impl FheRepositoryFactory for Repositories {
    fn fhe(&self, e3_id: &E3id) -> Repository<FheSnapshot> {
        Repository::new(self.store.scope(StoreKeys::fhe(e3_id)))
    }
}
