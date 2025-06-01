use data::{Repositories, Repository};
use e3_config::StoreKeys;
use events::E3id;

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
