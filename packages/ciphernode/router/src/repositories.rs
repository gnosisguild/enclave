use crate::{CommitteeMeta, E3RequestContextSnapshot, E3RequestRouterSnapshot};
use aggregator::{PlaintextAggregatorState, PublicKeyAggregatorState};
use data::{DataStore, Repository};
use enclave_core::E3id;
use fhe::FheSnapshot;
use keyshare::KeyshareState;
use sortition::SortitionModule;

pub struct Repositories {
    store: DataStore,
}

impl From<DataStore> for Repositories {
    fn from(value: DataStore) -> Self {
        Repositories { store: value }
    }
}
impl From<&DataStore> for Repositories {
    fn from(value: &DataStore) -> Self {
        Repositories {
            store: value.clone(),
        }
    }
}

impl Repositories {
    pub fn new(store: DataStore) -> Self {
        Repositories { store }
    }
}

impl<T> From<Repository<T>> for Repositories {
    fn from(value: Repository<T>) -> Self {
        let store: DataStore = value.into();
        store.into()
    }
}

impl Repositories {
    pub fn keyshare(&self, e3_id: &E3id) -> Repository<KeyshareState> {
        Repository::new(self.store.scope(format!("//keyshare/{e3_id}")))
    }

    pub fn plaintext(&self, e3_id: &E3id) -> Repository<PlaintextAggregatorState> {
        Repository::new(self.store.scope(format!("//plaintext/{e3_id}")))
    }

    pub fn publickey(&self, e3_id: &E3id) -> Repository<PublicKeyAggregatorState> {
        Repository::new(self.store.scope(format!("//publickey/{e3_id}")))
    }

    pub fn fhe(&self, e3_id: &E3id) -> Repository<FheSnapshot> {
        Repository::new(self.store.scope(format!("//fhe/{e3_id}")))
    }

    pub fn meta(&self, e3_id: &E3id) -> Repository<CommitteeMeta> {
        Repository::new(self.store.scope(format!("//meta/{e3_id}")))
    }

    pub fn context(&self, e3_id: &E3id) -> Repository<E3RequestContextSnapshot> {
        Repository::new(self.store.scope(format!("//context/{e3_id}")))
    }

    pub fn router(&self) -> Repository<E3RequestRouterSnapshot> {
        Repository::new(self.store.scope(format!("//router")))
    }

    pub fn sortition(&self) -> Repository<SortitionModule> {
        Repository::new(self.store.scope(format!("//sortition")))
    }
}

pub trait RepositoriesFactory {
    fn repositories(&self) -> Repositories;
}

impl RepositoriesFactory for DataStore {
    fn repositories(&self) -> Repositories {
        self.into()
    }
}

impl<T> RepositoriesFactory for Repository<T> {
    fn repositories(&self) -> Repositories {
        let store:DataStore = self.clone().into();
        store.repositories()
    }
}
