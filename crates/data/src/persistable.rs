// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use crate::{Get, Insert, Remove, Repository};
use actix::Recipient;
use anyhow::*;
use async_trait::async_trait;
use e3_events::{EventContext, EventContextManager, Sequenced};
use serde::{de::DeserializeOwned, Serialize};

pub trait PersistableData: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {}
impl<T> PersistableData for T where T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {}

/// AutoPersist enables a repository to generate a persistable container
#[async_trait]
pub trait AutoPersist<T>
where
    T: PersistableData,
{
    /// Load the data from the source into an auto persist container
    async fn load(&self) -> Result<Persistable<T>>;
    /// Create a new auto persist container and set some data on it to send back to the source
    fn send(&self, data: Option<T>) -> Persistable<T>;
    /// Load the data from the source into an auto persist container. If there is no persisted data then persist the given default data  
    async fn load_or_default(&self, default: T) -> Result<Persistable<T>>;
    /// Load the data from the source into an auto persist container. If there is no persisted data then persist the given default data  
    async fn load_or_else<F>(&self, f: F) -> Result<Persistable<T>>
    where
        F: Send + FnOnce() -> Result<T>;
}

#[async_trait]
impl<T> AutoPersist<T> for Repository<T>
where
    T: PersistableData,
{
    async fn load(&self) -> Result<Persistable<T>> {
        self.to_connector().load().await
    }

    fn send(&self, data: Option<T>) -> Persistable<T> {
        self.to_connector().send(data)
    }

    async fn load_or_default(&self, default: T) -> Result<Persistable<T>> {
        self.to_connector().load_or_default(default).await
    }

    async fn load_or_else<F>(&self, f: F) -> Result<Persistable<T>>
    where
        F: Send + FnOnce() -> Result<T>,
    {
        self.to_connector().load_or_else(f).await
    }
}

/// Connector to connect to store
#[derive(Clone, Debug)]
pub struct StoreConnector {
    pub key: Vec<u8>,
    pub get: Recipient<Get>,
    pub insert: Recipient<Insert>,
    pub remove: Recipient<Remove>,
}

impl StoreConnector {
    pub fn new(
        key: &[u8],
        get: &Recipient<Get>,
        insert: &Recipient<Insert>,
        remove: &Recipient<Remove>,
    ) -> Self {
        Self {
            key: key.to_owned(),
            get: get.clone(),
            insert: insert.clone(),
            remove: remove.clone(),
        }
    }
}

#[async_trait]
impl<T> AutoPersist<T> for StoreConnector
where
    T: PersistableData,
{
    async fn load(&self) -> Result<Persistable<T>> {
        Persistable::load(self.clone()).await
    }

    fn send(&self, data: Option<T>) -> Persistable<T> {
        Persistable::new(data, self.clone()).save()
    }

    async fn load_or_default(&self, default: T) -> Result<Persistable<T>> {
        Persistable::load_or_default(self.clone(), default).await
    }

    async fn load_or_else<F>(&self, f: F) -> Result<Persistable<T>>
    where
        F: Send + FnOnce() -> Result<T>,
    {
        Persistable::load_or_else(self.clone(), f).await
    }
}

/// A container that automatically persists its content every time it is mutated or changed.
#[derive(Debug)]
pub struct Persistable<T> {
    data: Option<T>,
    connector: StoreConnector,
    ctx: Option<EventContext<Sequenced>>,
}

impl<T> Persistable<T>
where
    T: PersistableData,
{
    /// Create a new container with the given data and connector
    pub fn new(data: Option<T>, connector: StoreConnector) -> Self {
        Self {
            data,
            connector,
            ctx: None,
        }
    }

    /// Load data from the store
    pub async fn load(connector: StoreConnector) -> Result<Self> {
        let data = Self::read_from_store(&connector).await?;
        Ok(Self::new(data, connector))
    }

    /// Load the data or save and sync the given default value
    pub async fn load_or_default(connector: StoreConnector, default: T) -> Result<Self> {
        let data = Self::read_from_store(&connector).await?.unwrap_or(default);
        let instance = Self::new(Some(data), connector);
        Ok(instance.save())
    }

    /// Load the data or save and sync the result of the given callback
    pub async fn load_or_else<F>(connector: StoreConnector, f: F) -> Result<Self>
    where
        F: FnOnce() -> Result<T>,
    {
        let data = Self::read_from_store(&connector)
            .await?
            .ok_or_else(|| anyhow!("Not found"))
            .or_else(|_| f())?;
        let instance = Self::new(Some(data), connector);
        Ok(instance.save())
    }

    async fn read_from_store(connector: &StoreConnector) -> Result<Option<T>> {
        let Some(bytes) = connector.get.send(Get::new(&connector.key)).await? else {
            return Ok(None);
        };
        if bytes == [0] {
            return Ok(None);
        }
        Ok(Some(bincode::deserialize(&bytes)?))
    }

    fn write_to_store(&self) {
        let Some(ref data) = self.data else {
            return;
        };
        let Result::Ok(serialized) = bincode::serialize(data) else {
            tracing::error!("Could not serialize value for persistable");
            return;
        };

        let msg = if let Some(ctx) = self.ctx.clone() {
            Insert::new_with_context(&self.connector.key, serialized, ctx)
        } else {
            Insert::new(&self.connector.key, serialized)
        };
        self.connector.insert.do_send(msg);
    }

    /// Save the data in the container to the store
    pub fn save(self) -> Self {
        self.write_to_store();
        self
    }

    /// Mutate the content if available or return an error
    pub fn try_mutate<F>(&mut self, mutator: F) -> Result<()>
    where
        F: FnOnce(T) -> Result<T>,
    {
        let content = self.data.clone().ok_or(anyhow!("Data has not been set"))?;
        self.data = Some(mutator(content)?);
        self.write_to_store();
        Ok(())
    }

    /// Set the data on both the persistable and the store
    pub fn set(&mut self, data: T) {
        self.data = Some(data);
        self.write_to_store();
    }

    /// Clear the data from both the persistable and the store
    pub fn clear(&mut self) {
        self.data = None;
        self.connector
            .remove
            .do_send(Remove::new(&self.connector.key));
    }

    /// Get the data currently stored on the container as an Option<T>
    pub fn get(&self) -> Option<T> {
        self.data.clone()
    }

    /// Get the data from the container or return an error
    pub fn try_get(&self) -> Result<T> {
        self.data
            .clone()
            .ok_or(anyhow!("Data was not set on container."))
    }

    /// Returns true if there is data on the container
    pub fn has(&self) -> bool {
        self.data.is_some()
    }
}

impl<T> EventContextManager for Persistable<T> {
    fn get_ctx(&self) -> Option<EventContext<Sequenced>> {
        self.ctx.clone()
    }

    fn set_ctx(&mut self, value: &EventContext<Sequenced>) {
        self.ctx = Some(value.clone())
    }
}
