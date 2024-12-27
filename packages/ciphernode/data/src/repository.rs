use std::{marker::PhantomData, ops::Deref};

use anyhow::Result;

use crate::DataStore;

#[derive(Debug)]
pub struct Repository<S> {
    /// store is currently set to be a scopeable key value store
    store: DataStore, // this could change and be abstracted if need be
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

impl<S> From<Repository<S>> for DataStore {
    fn from(value: Repository<S>) -> Self {
        value.store
    }
}

impl<T> From<&Repository<T>> for DataStore {
    fn from(value: &Repository<T>) -> Self {
        value.store.clone()
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

    pub async fn has(&self) -> bool {
        self.read().await.ok().flatten().is_some()
    }

    pub fn write(&self, value: &T) {
        self.store.write(value)
    }

    pub fn clear(&self) {
        self.store.write::<Option<T>>(None)
    }
}
