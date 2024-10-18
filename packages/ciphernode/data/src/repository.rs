use anyhow::Result;
use async_trait::async_trait;

use crate::DataStore;

#[async_trait]
pub trait Repository {
    type State: for<'de> serde::Deserialize<'de> + serde::Serialize;
    fn store(&self) -> DataStore;

    async fn read(&self) -> Result<Option<Self::State>> {
        self.store().read().await
    }

    fn write(&self, value: &Self::State) {
        self.store().write(value)
    }
}
