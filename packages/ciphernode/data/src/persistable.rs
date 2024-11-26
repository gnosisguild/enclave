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

#[async_trait]
pub trait WithPersistable<T>
where
    T: PersistableData,
{
    async fn synced(self) -> Result<Persistable<T>>;
    fn persistable(self, data: Option<T>) -> Persistable<T>;
}

#[async_trait]
impl<T> WithPersistable<T> for Repository<T>
where
    T: PersistableData,
{
    async fn synced(self) -> Result<Persistable<T>> {
        Ok(Persistable::load(self).await?)
    }

    fn persistable(self, data: Option<T>) -> Persistable<T> {
        Persistable::new(data, self)
    }
}

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

    pub fn mutate<F>(&mut self, mutator: F)
    where
        F: FnOnce(Option<T>) -> T,
    {
        let current = std::mem::take(&mut self.data);
        self.data = Some(mutator(current));
        self.checkpoint();
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
