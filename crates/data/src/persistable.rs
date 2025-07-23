// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{Checkpoint, FromSnapshotWithParams, Repository, Snapshot};
use anyhow::*;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

pub trait PersistableData: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {}
impl<T> PersistableData for T where T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {}

/// AutoPersist enables a repository to generate a persistable container
#[async_trait]
pub trait AutoPersist<T>
where
    T: PersistableData,
{
    /// Load the data from the repository into an auto persist container
    async fn load(&self) -> Result<Persistable<T>>;
    /// Create a new auto persist container and set some data on it to send back to the repository
    fn send(&self, data: Option<T>) -> Persistable<T>;
    /// Load the data from the repository into an auto persist container. If there is no persisted data then persist the given default data  
    async fn load_or_default(&self, default: T) -> Result<Persistable<T>>;
    /// Load the data from the repository into an auto persist container. If there is no persisted data then persist the given default data  
    async fn load_or_else<F>(&self, f: F) -> Result<Persistable<T>>
    where
        F: Send + FnOnce() -> Result<T>;
}

#[async_trait]
impl<T> AutoPersist<T> for Repository<T>
where
    T: PersistableData,
{
    /// Load the data from the repository into an auto persist container
    async fn load(&self) -> Result<Persistable<T>> {
        Ok(Persistable::load(self).await?)
    }

    /// Create a new auto persist container and set some data on it to send back to the repository
    fn send(&self, data: Option<T>) -> Persistable<T> {
        Persistable::new(data, self).save()
    }

    /// Load the data from the repository into an auto persist container. If there is no persisted data then persist the given default data  
    async fn load_or_default(&self, default: T) -> Result<Persistable<T>> {
        Ok(Persistable::load_or_default(self, default).await?)
    }

    /// Load the data from the repository into an auto persist container. If there is no persisted data then persist the result of the callback
    async fn load_or_else<F>(&self, f: F) -> Result<Persistable<T>>
    where
        F: Send + FnOnce() -> Result<T>,
    {
        Ok(Persistable::load_or_else(self, f).await?)
    }
}

/// A container that automatically persists it's content every time it is mutated or changed.
#[derive(Debug)]
pub struct Persistable<T> {
    data: Option<T>,
    repo: Repository<T>,
}

impl<T> Persistable<T>
where
    T: PersistableData,
{
    /// Create a new container with the given option data and repository
    pub fn new(data: Option<T>, repo: &Repository<T>) -> Self {
        Self {
            data,
            repo: repo.clone(),
        }
    }

    /// Load data from the repository to the container
    pub async fn load(repo: &Repository<T>) -> Result<Self> {
        let data = repo.read().await?;

        Ok(Self::new(data, repo))
    }

    /// Load the data from the repo or save and sync the given default value
    pub async fn load_or_default(repo: &Repository<T>, default: T) -> Result<Self> {
        let instance = Self::new(Some(repo.read().await?.unwrap_or(default)), repo);

        Ok(instance.save())
    }

    /// Load the data from the repo or save and sync the result of the given callback
    pub async fn load_or_else<F>(repo: &Repository<T>, f: F) -> Result<Self>
    where
        F: FnOnce() -> Result<T>,
    {
        let data = repo
            .read()
            .await?
            .ok_or_else(|| anyhow!("Not found"))
            .or_else(|_| f())?;

        let instance = Self::new(Some(data), repo);
        Ok(instance.save())
    }

    /// Save the data in the container to the database
    pub fn save(self) -> Self {
        self.checkpoint();
        self
    }

    /// Mutate the content if it is available or return an error if either the mutator function
    /// fails or if the data has not been set.
    pub fn try_mutate<F>(&mut self, mutator: F) -> Result<()>
    where
        F: FnOnce(T) -> Result<T>,
    {
        let content = self.data.clone().ok_or(anyhow!("Data has not been set"))?;
        self.data = Some(mutator(content)?);
        self.checkpoint();
        Ok(())
    }

    /// Set the data on both the persistable and the repository.
    pub fn set(&mut self, data: T) {
        self.data = Some(data);
        self.checkpoint();
    }

    /// Clear the data from both the persistable and the repository.
    pub fn clear(&mut self) {
        self.data = None;
        self.clear_checkpoint();
    }

    /// Get the data currently stored on the container as an Option<T>
    pub fn get(&self) -> Option<T> {
        self.data.clone()
    }

    /// Get the data from the container or return an error.
    pub fn try_get(&self) -> Result<T> {
        self.data
            .clone()
            .ok_or(anyhow!("Data was not set on container."))
    }

    /// Returns true if there is data on the container and false if there is not.
    pub fn has(&self) -> bool {
        self.data.is_some()
    }

    /// Get an immutable reference to the data on the container if the data is not set on the
    /// container return an error
    pub fn try_with<F, U>(&self, f: F) -> Result<U>
    where
        F: FnOnce(&T) -> Result<U>,
    {
        match &self.data {
            Some(data) => f(data),
            None => Err(anyhow!("Data was not set on container.")),
        }
    }
}

impl<T> Snapshot for Persistable<T>
where
    T: PersistableData,
{
    type Snapshot = T;
    fn snapshot(&self) -> Result<Self::Snapshot> {
        Ok(self
            .data
            .clone()
            .ok_or(anyhow!("No data stored on container"))?)
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
        Ok(Persistable::new(Some(snapshot), &params))
    }
}

#[cfg(test)]
mod tests {
    use crate::{AutoPersist, DataStore, GetLog, InMemStore, Repository};
    use actix::{Actor, Addr};
    use anyhow::{anyhow, Result};

    fn get_repo<T>() -> (Repository<T>, Addr<InMemStore>) {
        let addr = InMemStore::new(true).start();
        let store = DataStore::from(&addr).scope("/");
        let repo: Repository<T> = Repository::new(store);
        (repo, addr)
    }

    #[actix::test]
    async fn persistable_loads_with_default() -> Result<()> {
        let (repo, addr) = get_repo::<Vec<String>>();
        let container = repo
            .clone()
            .load_or_default(vec!["berlin".to_string()])
            .await?;

        assert_eq!(addr.send(GetLog).await?.len(), 1);
        assert_eq!(repo.read().await?, Some(vec!["berlin".to_string()]));
        assert_eq!(container.get(), Some(vec!["berlin".to_string()]));
        Ok(())
    }

    #[actix::test]
    async fn persistable_loads_with_default_override() -> Result<()> {
        let (repo, _) = get_repo::<Vec<String>>();
        repo.write(&vec!["berlin".to_string()]);
        let container = repo
            .clone()
            .load_or_default(vec!["amsterdam".to_string()])
            .await?;

        assert_eq!(repo.read().await?, Some(vec!["berlin".to_string()]));
        assert_eq!(container.get(), Some(vec!["berlin".to_string()]));
        Ok(())
    }

    #[actix::test]
    async fn persistable_load() -> Result<()> {
        let (repo, _) = get_repo::<Vec<String>>();
        repo.write(&vec!["berlin".to_string()]);
        let container = repo.clone().load().await?;

        assert_eq!(repo.read().await?, Some(vec!["berlin".to_string()]));
        assert_eq!(container.get(), Some(vec!["berlin".to_string()]));
        Ok(())
    }

    #[actix::test]
    async fn persistable_send() -> Result<()> {
        let (repo, _) = get_repo::<Vec<String>>();
        repo.write(&vec!["amsterdam".to_string()]);
        let container = repo.clone().send(Some(vec!["berlin".to_string()]));

        assert_eq!(repo.read().await?, Some(vec!["berlin".to_string()]));
        assert_eq!(container.get(), Some(vec!["berlin".to_string()]));
        Ok(())
    }

    #[actix::test]
    async fn persistable_mutate() -> Result<()> {
        let (repo, addr) = get_repo::<Vec<String>>();

        let mut container = repo.clone().send(Some(vec!["berlin".to_string()]));

        container.try_mutate(|mut list| {
            list.push(String::from("amsterdam"));
            Ok(list)
        })?;

        assert_eq!(
            repo.read().await?,
            Some(vec!["berlin".to_string(), "amsterdam".to_string()])
        );

        assert_eq!(addr.send(GetLog).await?.len(), 2);

        Ok(())
    }

    #[actix::test]
    async fn test_clear_persistable() -> Result<()> {
        let (repo, _) = get_repo::<Vec<String>>();
        let repo_ref = &repo;
        let mut container = repo_ref.send(Some(vec!["berlin".to_string()]));

        assert!(container.has());
        container.clear();
        assert!(!container.has());
        assert_eq!(repo_ref.read().await?, None);
        Ok(())
    }

    #[actix::test]
    async fn test_set_persistable() -> Result<()> {
        let (repo, _) = get_repo::<Vec<String>>();
        let mut container = repo.clone().send(None);

        container.set(vec!["amsterdam".to_string()]);

        assert!(container.has());
        assert_eq!(repo.read().await?, Some(vec!["amsterdam".to_string()]));
        Ok(())
    }

    #[actix::test]
    async fn test_try_get_with_data() -> Result<()> {
        let (repo, _) = get_repo::<Vec<String>>();
        let container = repo.clone().send(Some(vec!["berlin".to_string()]));

        let result = container.try_get()?;
        assert_eq!(result, vec!["berlin".to_string()]);
        Ok(())
    }

    #[actix::test]
    async fn test_try_get_without_data() {
        let (repo, _) = get_repo::<Vec<String>>();
        let container = repo.clone().send(None);

        assert!(container.try_get().is_err());
    }

    #[actix::test]
    async fn test_try_with_success() -> Result<()> {
        let (repo, _) = get_repo::<Vec<String>>();
        let container = repo.clone().send(Some(vec!["berlin".to_string()]));

        let length = container.try_with(|data| Ok(data.len()))?;
        assert_eq!(length, 1);
        Ok(())
    }

    #[actix::test]
    async fn test_try_with_failure() {
        let (repo, _) = get_repo::<Vec<String>>();
        let container = repo.clone().send(None);

        let result = container.try_with(|data| Ok(data.len()));
        assert!(result.is_err());
    }

    #[actix::test]
    async fn test_try_mutate_failure() {
        let (repo, _) = get_repo::<Vec<String>>();
        let mut container = repo.clone().send(None);

        let result = container.try_mutate(|mut list| {
            list.push(String::from("amsterdam"));
            Ok(list)
        });
        assert!(result.is_err());
    }

    #[actix::test]
    async fn test_mutate_with_error() -> Result<()> {
        let (repo, _) = get_repo::<Vec<String>>();
        let mut container = repo.clone().send(Some(vec!["berlin".to_string()]));

        let result =
            container.try_mutate(|_| -> Result<Vec<String>> { Err(anyhow!("Mutation failed")) });

        assert!(result.is_err());
        // Original data should remain unchanged
        assert_eq!(container.try_get()?, vec!["berlin".to_string()]);
        Ok(())
    }

    #[actix::test]
    async fn test_load_or_else_success_with_empty_repo() -> Result<()> {
        let (repo, _) = get_repo::<Vec<String>>();

        let container = repo
            .clone()
            .load_or_else(|| Ok(vec!["amsterdam".to_string()]))
            .await?;

        assert_eq!(container.try_get()?, vec!["amsterdam".to_string()]);
        assert_eq!(repo.read().await?, Some(vec!["amsterdam".to_string()]));
        Ok(())
    }

    #[actix::test]
    async fn test_load_or_else_skips_callback_when_data_exists() -> Result<()> {
        let (repo, _) = get_repo::<Vec<String>>();
        repo.write(&vec!["berlin".to_string()]);

        let container = repo
            .clone()
            .load_or_else(|| {
                panic!("This callback should not be called!");
                #[allow(unreachable_code)]
                Ok(vec!["amsterdam".to_string()])
            })
            .await?;

        assert_eq!(container.try_get()?, vec!["berlin".to_string()]);
        Ok(())
    }

    #[actix::test]
    async fn test_load_or_else_propagates_callback_error() -> Result<()> {
        let (repo, _) = get_repo::<Vec<String>>();

        let result = repo
            .clone()
            .load_or_else(|| Err(anyhow!("Failed to create default data")))
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to create default data"));
        assert_eq!(repo.read().await?, None);
        Ok(())
    }

    #[actix::test]
    async fn test_load_or_else_custom_error_message() -> Result<()> {
        let (repo, _) = get_repo::<Vec<String>>();
        let error_msg = "Custom initialization error";

        let result = repo.load_or_else(|| Err(anyhow!(error_msg))).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(error_msg));
        Ok(())
    }
}
