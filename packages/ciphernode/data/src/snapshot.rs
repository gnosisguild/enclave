use crate::DataStore;
use anyhow::Result;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

/// This trait enables the self type to report their state snapshot
pub trait Snapshot
where
    Self: Sized,
{
    /// The Snapshot should represent all the dynamic data managed within the Actor or Object
    ///
    /// The state must be serializable so that it can be stored as a value
    type Snapshot: Serialize + DeserializeOwned;

    /// Return the Snapshot object for the implementor
    fn snapshot(&self) -> Self::Snapshot;
}

/// This trait enables the self type to checkpoint its state
pub trait Checkpoint: Snapshot {
    /// Declare the DataStore instance available on the object
    fn get_store(&self) -> DataStore;

    /// Write the current snapshot to the DataStore provided by `get_store()` at the object's id returned by `get_id()`
    fn checkpoint(&self) {
        self.get_store().write(self.snapshot());
    }
}

/// Enable the self type to be reconstituted from the parameters coupled with the Snapshot
#[async_trait]
pub trait FromSnapshotWithParams: Snapshot {
    type Params: Send + 'static;

    /// Return an instance of the persistable object at the state given by the snapshot
    /// This method is async because there may be subobjects that require hydration from the store
    async fn from_snapshot(params: Self::Params, snapshot: Self::Snapshot) -> Result<Self>;
}

#[async_trait]
pub trait FromSnapshot: Snapshot {
    /// Return an instance of the persistable object at the state given by the snapshot
    /// This method is async because there may be subobjects that require hydration from the store
    async fn from_snapshot(snapshot: Self::Snapshot) -> Result<Self>;
}
