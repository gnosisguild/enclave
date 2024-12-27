use crate::{DataStore, Repository};

// TODO: Naming here is confusing
pub struct Repositories {
    pub store: DataStore,
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
