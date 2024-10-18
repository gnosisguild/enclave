use std::{marker::PhantomData, ops::Deref};

use anyhow::Result;

use crate::DataStore;

pub struct Repository<S> {
    store: DataStore,
    _p: PhantomData<S>,
}

impl<S> Repository<S> {
    pub fn new(store: DataStore) -> Self {
        Self {
            store,
            _p: PhantomData,
        }
    }
}

impl<S> Deref for Repository<S>{
    type Target = DataStore;
    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl<S> From<Repository<S>> for DataStore {
    fn from(value: Repository<S>) -> Self {
        value.store
    }
}

/// Clone without phantom data
impl<S> Clone for Repository<S> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            _p: PhantomData,
        }
    }
}

impl<T> Repository<T>
where
    T: for<'de> serde::Deserialize<'de> + serde::Serialize,
{
    pub async fn read(&self) -> Result<Option<T>> {
        self.store.read().await
    }

    pub fn write(&self, value: &T) {
        self.store.write(value)
    }
}
