use crate::{CommitteeMeta, E3RequestContextSnapshot, E3RequestRouterSnapshot};
use aggregator::{PlaintextAggregatorState, PublicKeyAggregatorState};
use config::StoreKeys;
use data::{DataStore, Repository};
use enclave_core::E3id;
use evm::EvmEventReaderState;
use fhe::FheSnapshot;
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
    pub fn keyshare(&self, e3_id: &E3id) -> Repository<Vec<u8>> {
        Repository::new(self.store.scope(StoreKeys::keyshare(e3_id)))
    }

    pub fn plaintext(&self, e3_id: &E3id) -> Repository<PlaintextAggregatorState> {
        Repository::new(self.store.scope(StoreKeys::plaintext(e3_id)))
    }

    pub fn publickey(&self, e3_id: &E3id) -> Repository<PublicKeyAggregatorState> {
        Repository::new(self.store.scope(StoreKeys::publickey(e3_id)))
    }

    pub fn fhe(&self, e3_id: &E3id) -> Repository<FheSnapshot> {
        Repository::new(self.store.scope(StoreKeys::fhe(e3_id)))
    }

    pub fn meta(&self, e3_id: &E3id) -> Repository<CommitteeMeta> {
        Repository::new(self.store.scope(StoreKeys::meta(e3_id)))
    }

    pub fn context(&self, e3_id: &E3id) -> Repository<E3RequestContextSnapshot> {
        Repository::new(self.store.scope(StoreKeys::context(e3_id)))
    }

    pub fn router(&self) -> Repository<E3RequestRouterSnapshot> {
        Repository::new(self.store.scope(StoreKeys::router()))
    }

    pub fn sortition(&self) -> Repository<SortitionModule> {
        Repository::new(self.store.scope(StoreKeys::sortition()))
    }

    pub fn eth_private_key(&self) -> Repository<Vec<u8>> {
        Repository::new(self.store.scope(StoreKeys::eth_private_key()))
    }

    pub fn libp2p_keypair(&self) -> Repository<Vec<u8>> {
        Repository::new(self.store.scope(StoreKeys::libp2p_keypair()))
    }

    pub fn enclave_sol_reader(&self, chain_id: u64) -> Repository<EvmEventReaderState> {
        Repository::new(self.store.scope(StoreKeys::enclave_sol_reader(chain_id)))
    }
    pub fn ciphernode_registry_reader(&self, chain_id: u64) -> Repository<EvmEventReaderState> {
        Repository::new(
            self.store
                .scope(StoreKeys::ciphernode_registry_reader(chain_id)),
        )
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
        let store: DataStore = self.into();
        store.repositories()
    }
}
