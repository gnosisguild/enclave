use crate::{Checkpoint, FromSnapshotWithParams, Repository, Snapshot};
use anyhow::*;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

pub trait PersistableData:
    Serialize + DeserializeOwned + Clone + Default + Send + Sync + 'static
{
}
impl<T> PersistableData for T where
    T: Serialize + DeserializeOwned + Clone + Default + Send + Sync + 'static
{
}

/// AutoPersist enables a repository to generate a persistable container
#[async_trait]
pub trait AutoPersist<T>
where
    T: PersistableData,
{
    /// Load the data from the repository into an auto persist container
    async fn sync_load(self) -> Result<Persistable<T>>;
    /// Create a new autosync container and set some data on it to sync back to the db
    fn sync_new(self, data: Option<T>) -> Persistable<T>;
    /// Load the data from the repository into an auto persist container if there is no persisted data then persist the given default data  
    async fn sync_or_default(self, default: T) -> Result<Persistable<T>>;
}

#[async_trait]
impl<T> AutoPersist<T> for Repository<T>
where
    T: PersistableData,
{
    async fn sync_load(self) -> Result<Persistable<T>> {
        Ok(Persistable::load(self).await?)
    }

    fn sync_new(self, data: Option<T>) -> Persistable<T> {
        Persistable::new(data, self).save()
    }

    async fn sync_or_default(self, default: T) -> Result<Persistable<T>> {
        Ok(Persistable::load_or_default(self, default).await?)
    }
}

/// A container that automatically persists it's content every time it is mutated
#[derive(Debug)]
pub struct Persistable<T> {
    data: Option<T>,
    repo: Repository<T>,
}

impl<T> Persistable<T>
where
    T: PersistableData,
{
    pub fn new(data: Option<T>, repo: Repository<T>) -> Self {
        Self { data, repo }
    }

    pub async fn load(repo: Repository<T>) -> Result<Self> {
        let data = repo.read().await?;

        Ok(Self { data, repo })
    }

    pub async fn load_or_default(repo: Repository<T>, default: T) -> Result<Self> {
        Ok(Self {
            data: Some(repo.read().await?.unwrap_or(default)),
            repo,
        })
    }

    pub fn save(self) -> Self {
        self.checkpoint();
        self
    }

    /// If the content is available it will be mutated with the mutator function. NOTE: If the content is
    /// not available nothing will happen.
    pub fn mutate<F>(&mut self, mutator: F)
    where
        F: FnOnce(T) -> T,
    {
        let current = std::mem::take(&mut self.data);
        self.data = current.map(mutator);
        self.checkpoint();
    }

    /// Mutate the content if it is available or return an error if not.
    pub fn try_mutate<F>(&mut self, mutator: F) -> Result<()>
    where
        F: FnOnce(T) -> T,
    {
        let current = std::mem::take(&mut self.data);
        if current.is_none() {
            self.data = None; // probably not necessary but just incase
            return Err(anyhow!("Data has not been set"));
        }
        self.data = current.map(mutator);
        self.checkpoint();
        Ok(())
    }

    pub fn set(&mut self, data: T) {
        self.data = Some(data);
        self.checkpoint();
    }

    pub fn clear(&mut self) {
        self.data = None;
        self.checkpoint();
    }

    pub fn get(&self) -> Option<T> {
        self.data.clone()
    }

    pub fn has(&self) -> bool {
        self.data.is_some()
    }

    pub fn with<F, U>(&self, default: U, f: F) -> U
    where
        F: FnOnce(&T) -> U,
    {
        match &self.data {
            Some(data) => f(data),
            None => default,
        }
    }

    pub fn try_with<F, U>(&self, default: U, f: F) -> Result<U>
    where
        F: FnOnce(&T) -> Result<U>,
    {
        match &self.data {
            Some(data) => f(data),
            None => Ok(default),
        }
    }
}

impl<T> Snapshot for Persistable<T>
where
    T: PersistableData,
{
    type Snapshot = T;
    fn snapshot(&self) -> Self::Snapshot {
        self.data.clone().unwrap_or_default()
    }
}

impl<T> Checkpoint for Persistable<T>
where
    T: PersistableData,
{
    fn repository(&self) -> &Repository<Self::Snapshot> {
        &self.repo
    }
}

#[async_trait]
impl<T> FromSnapshotWithParams for Persistable<T>
where
    T: PersistableData,
{
    type Params = Repository<T>;
    async fn from_snapshot(params: Repository<T>, snapshot: T) -> Result<Self> {
        Ok(Persistable::new(Some(snapshot), params))
    }
}
